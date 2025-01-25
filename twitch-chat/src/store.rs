use std::{
    collections::BTreeSet,
    fs::File,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
};

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use twitch_api::events::ws::NotificationMessageEvent;

pub struct Store {
    directory: PathBuf,
    files: BTreeSet<NaiveDate>,
    today: Vec<Event>,
    today_file: Option<File>,
    search: Option<Search>,
}

impl Store {
    pub fn init(path: PathBuf) -> Result<Self> {
        let mut store = Self {
            directory: path,
            files: BTreeSet::new(),
            today: Vec::new(),
            today_file: None,
            search: None,
        };

        store.update_files()?;
        store.update_today()?;

        Ok(store)
    }

    fn update_files(&mut self) -> Result<()> {
        self.files = self
            .directory
            .read_dir()
            .context("read storage directory")?
            .filter_map(|entry| {
                let entry = match entry.context("read storage directory entry") {
                    Ok(it) => it,
                    Err(err) => return Some(Err(err)),
                };
                entry
                    .file_name()
                    .to_str()?
                    .strip_suffix(".json")?
                    .parse()
                    .ok()
                    .map(Ok)
            })
            .collect::<Result<_>>()?;
        dbg!(&self.files);
        Ok(())
    }

    fn file_path(&self, date: NaiveDate) -> PathBuf {
        self.directory.join(format!("{date}.json"))
    }

    fn load_file(&self, date: NaiveDate) -> Result<impl Iterator<Item = Result<Event>>> {
        let events = File::open(self.file_path(date)).context("open storage file")?;
        let events = BufReader::new(events).lines().map(|line| {
            let line = line.context("read storage file")?;
            let event = serde_json::from_str(&line).context("parse stored event")?;
            Ok(event)
        });
        Ok(events)
    }

    fn update_today(&mut self) -> Result<()> {
        let today = chrono::Utc::now()
            .with_timezone(&chrono_tz::Europe::Berlin)
            .date_naive();
        let events = if self.files.contains(&today) {
            self.load_file(today)?.collect::<Result<_>>()?
        } else {
            Vec::new()
        };
        self.today = events;

        self.today_file = Some(
            File::options()
                .append(true)
                .create(true)
                .open(self.file_path(today))
                .context("failed to open today storage file")?,
        );

        Ok(())
    }

    pub fn push(&mut self, event: Event) -> Result<()> {
        let mut json = serde_json::to_string(&event).context("encode storage event")?;
        json.push('\n');
        self.today_file
            .as_mut()
            .unwrap()
            .write_all(json.as_bytes())
            .context("write storage event")?;
        self.today.push(event);
        Ok(())
    }

    pub fn events(&self) -> &[Event] {
        self.search.as_ref().map_or(&self.today, |s| &s.results)
    }
}

struct Search {
    query: String,
    results: Vec<Event>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Event {
    Started {
        started_at: DateTime<Utc>,
    },
    Message {
        sent_at: DateTime<Utc>,
        user_login: String,
        text: String,
    },
    Notification {
        timestamp: DateTime<Utc>,
        event: NotificationMessageEvent,

        #[serde(default, skip_serializing_if = "Value::is_null")]
        extra: Value,
    },
}
