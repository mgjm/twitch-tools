use std::{
    collections::BTreeSet,
    fs::File,
    io::{BufRead, BufReader, Write},
    num::NonZeroUsize,
    ops::Bound,
    path::PathBuf,
    sync::Arc,
};

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
use nucleo::{
    Nucleo,
    pattern::{CaseMatching, Normalization},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Notify;
use twitch_api::events::{
    chat::{message::ChatMessage, notification::ChatNotification},
    follow::Follow,
    stream::{StreamOffline, StreamOnline},
    ws::NotificationMessageEvent,
};

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
            .with_timezone(crate::timezone())
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

    pub fn events_len(&self) -> usize {
        match &self.search {
            Some(search) => search
                .nucleo
                .snapshot()
                .matched_item_count()
                .try_into()
                .unwrap(),
            None => self.today.len(),
        }
    }

    pub fn events(&self, offset: &mut Option<NonZeroUsize>) -> impl Iterator<Item = &Event> {
        enum Either<A, B> {
            Left(A),
            Right(B),
        }

        impl<A, B> Iterator for Either<A, B>
        where
            A: Iterator,
            B: Iterator<Item = A::Item>,
        {
            type Item = A::Item;

            fn next(&mut self) -> Option<Self::Item> {
                match self {
                    Either::Left(iter) => iter.next(),
                    Either::Right(iter) => iter.next(),
                }
            }
        }

        match &self.search {
            Some(search) => {
                let snapshot = search.nucleo.snapshot();
                let len = snapshot.matched_item_count().try_into().unwrap();
                if matches!(offset, Some(offset) if offset.get() >= len) {
                    *offset = None;
                }
                let start = match offset {
                    Some(offset) => Bound::Included(len.saturating_sub(offset.get()) as u32),
                    None => Bound::Unbounded,
                };
                Either::Left(
                    snapshot
                        .matched_items((start, Bound::Unbounded))
                        .map(|item| item.data),
                )
            }
            None => {
                if matches!(offset, Some(offset) if offset.get() >= self.today.len()) {
                    *offset = None;
                }
                Either::Right(
                    if let Some(offset) = offset {
                        &self.today[..offset.get()]
                    } else {
                        &self.today
                    }
                    .iter()
                    .rev(),
                )
            }
        }
    }

    pub fn start_search(&mut self, query: &str) {
        if query.is_empty() {
            self.search = None;
            return;
        }

        if let Some(search) = &mut self.search {
            if search.query == query {
                return;
            }

            let append = query.starts_with(search.query.as_str());
            search.query = query.into();
            search.nucleo.pattern.reparse(
                1,
                query,
                CaseMatching::Smart,
                Normalization::Smart,
                append,
            );
        } else {
            let notify = Arc::new(Notify::new());

            let mut nucleo = {
                let notify = Arc::downgrade(&notify);
                nucleo::Nucleo::new(
                    nucleo::Config::DEFAULT,
                    Arc::new(move || {
                        if let Some(notify) = notify.upgrade() {
                            notify.notify_one();
                        }
                    }),
                    None,
                    Event::NUM_COLUMNS,
                )
            };

            nucleo
                .pattern
                .reparse(1, query, CaseMatching::Smart, Normalization::Smart, false);

            for event in self.today.iter().rev() {
                nucleo.injector().push(event.clone(), |event, columns| {
                    event.fill_columns(columns).unwrap();
                });
            }

            self.search = Some(Search {
                query: query.into(),
                nucleo,
                notify,
            });
        }
    }

    pub fn tick(&mut self) {
        if let Some(search) = &mut self.search {
            search.nucleo.tick(10);
        }
    }

    pub fn search_changed(&self) -> impl Future<Output = ()> + 'static {
        let notify = self.search.as_ref().map(|s| s.notify.clone());
        async {
            if let Some(notify) = notify {
                notify.notified().await
            } else {
                std::future::pending().await
            }
        }
    }
}

struct Search {
    query: String,
    nucleo: Nucleo<Event>,
    notify: Arc<Notify>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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

impl Event {
    const NUM_COLUMNS: u32 = 2;

    fn fill_columns(&self, columns: &mut [nucleo::Utf32String]) -> Result<()> {
        let [user, text] = columns else {
            anyhow::bail!("{} colomns", columns.len());
        };

        [*user, *text] = match self {
            Event::Started { .. } => [Default::default(), "chat started".into()],
            Event::Message {
                user_login, text, ..
            } => [user_login.as_str().into(), text.as_str().into()],
            Event::Notification { event, .. } => {
                let notification = event;
                if let Some(message) = notification.parse::<ChatMessage>()? {
                    [
                        message.chatter_user_name.into(),
                        message.message.text.into(),
                    ]
                } else if let Some(notification) = notification.parse::<ChatNotification>()? {
                    [
                        notification.chatter_user_name.into(),
                        notification.message.text.into(),
                    ]
                } else if let Some(follow) = notification.parse::<Follow>()? {
                    [follow.user_name.into(), "has followd you".into()]
                } else if let Some(_online) = notification.parse::<StreamOnline>()? {
                    [Default::default(), "stream went online".into()]
                } else if let Some(_offline) = notification.parse::<StreamOffline>()? {
                    [Default::default(), "stream went offline".into()]
                } else {
                    Default::default()
                }
            }
        };

        Ok(())
    }
}
