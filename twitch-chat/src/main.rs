use std::{io, sync::OnceLock};

use anyhow::{Context, Result};
use chrono_tz::Tz;
use clap::Parser;
use config::Keybindings;
use crossterm::event;
use tokio::task::LocalSet;
use twitch::Subscriptions;
use twitch_api::{
    auth::{self, Scope},
    client::Client,
    events::subscription::{DeleteSubscriptionRequest, GetSubscriptionsRequest},
    secret::Secret,
    user::UsersRequest,
};

mod chat;
mod cmd;
mod config;
mod sound_system;
mod store;
mod twitch;

#[derive(Debug, Parser)]
#[clap(version)]
/// Twitch chat in the terminal
enum Cmd {
    Auth(auth::Auth),
    Run(cmd::Run),
    #[clap(subcommand)]
    Eventsub(cmd::Eventsub),
}

fn main() -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(LocalSet::new().run_until(run()))
}

async fn run() -> Result<()> {
    let cmd = Cmd::parse();

    match cmd {
        Cmd::Auth(cmd) => {
            cmd.run([
                Scope::UserReadChat,
                Scope::UserWriteChat,
                Scope::ModeratorManageAnnouncements,
                Scope::ModeratorReadFollowers,
            ])
            .await
        }
        Cmd::Run(cmd) => cmd.run().await,
        Cmd::Eventsub(cmd) => cmd.run().await,
    }
}

static TIMEZONE: OnceLock<Tz> = OnceLock::new();

fn timezone() -> &'static Tz {
    TIMEZONE.get().expect("timezone not set")
}

impl cmd::Run {
    async fn run(&self) -> Result<()> {
        let config = crate::config::Config::open(&self.config)?;
        anyhow::ensure!(
            TIMEZONE.set(config.timezone).is_ok(),
            "timezone already set",
        );

        let mut keybindings = Keybindings::default();
        keybindings.extend(config.keybindings);

        let sound_system = sound_system::SoundSystem::init(config.outputs, config.sounds)?;

        eprintln!("sound system initialized");

        let store = crate::store::Store::init(config.store.path)?;

        let mut client = Client::new().authenticated_from_env()?;

        let user = client
            .send(&UsersRequest::me())
            .await
            .context("fetch user me")?
            .into_user()
            .context("missing me user")?;
        eprintln!("user id: {:?}", user.id);

        let (subsciptions, ws) = Subscriptions::subscribe(&mut client, &user).await?;

        let terminal = ratatui::init();
        let tty_mode_guard = TtyModes::enable();
        let run_result = chat::run(
            terminal,
            keybindings,
            store,
            &mut client,
            user,
            ws,
            sound_system,
        )
        .await;

        drop(tty_mode_guard);
        ratatui::restore();

        subsciptions.unsubscribe(&mut client).await?;

        run_result
    }
}

impl cmd::Eventsub {
    async fn run(self) -> Result<()> {
        let mut client = Client::new().authenticated_from_env()?;

        match self {
            Self::List {} => {
                let res = client
                    .send(&GetSubscriptionsRequest {
                        ..Default::default()
                    })
                    .await
                    .context("get subscriptions")?;
                eprintln!("{res:#?}");
            }
            Self::Delete { all, id } => {
                let ids = if all {
                    let res = client
                        .send(&GetSubscriptionsRequest {
                            ..Default::default()
                        })
                        .await
                        .context("get subscriptions")?;

                    res.data.into_iter().map(|i| i.id).collect()
                } else {
                    Vec::from_iter(id.map(Secret::new))
                };

                let num_ids = ids.len();
                for id in ids {
                    client
                        .send(&DeleteSubscriptionRequest { id })
                        .await
                        .context("delete subscription")?;
                }

                eprintln!("deleted {num_ids} ids",);
            }
        }

        Ok(())
    }
}

#[must_use]
struct TtyModes(());

impl TtyModes {
    fn enable() -> Self {
        crossterm::execute!(
            io::stdout(),
            event::EnableFocusChange,
            event::EnableMouseCapture,
        )
        .expect("enable tty modes");
        Self(())
    }
}

impl Drop for TtyModes {
    fn drop(&mut self) {
        if let Err(err) = crossterm::execute!(
            io::stdout(),
            event::DisableFocusChange,
            event::DisableMouseCapture,
        ) {
            eprintln!("failed to disable tty modes: {err}");
        }
    }
}
