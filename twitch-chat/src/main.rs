use std::hash::{DefaultHasher, Hash, Hasher};

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::style::{Color, Stylize};
use tokio::task::LocalSet;
use twitch_api::{
    auth::{self, Scope},
    channel::{Channel, ChannelsRequest},
    chat::ChatColorsRequest,
    client::Client,
    events::{
        chat::{ChatMessage, ChatMessageCondition},
        follow::{Follow, FollowCondition},
        stream::{StreamOffline, StreamOfflineCondition, StreamOnline, StreamOnlineCondition},
        subscription::{
            CreateSubscriptionRequest, DeleteSubscriptionRequest, GetSubscriptionsRequest,
            TransportRequest,
        },
        ws::WebSocket,
    },
    follower::ChannelFollowersRequest,
    secret::Secret,
    stream::{Stream, StreamsRequest},
    user::UsersRequest,
};

use crate::config::Event;

mod cmd;
mod config;
mod sound_system;

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

impl cmd::Run {
    async fn run(&self) -> Result<()> {
        let config = crate::config::Config::open(&self.config)?;

        let mut sound_system = sound_system::SoundSystem::init(config.outputs, config.sounds)?;

        eprintln!("sound system initialized");

        let mut client = Client::new().authenticated_from_env()?;

        let user = client
            .send(&UsersRequest::me())
            .await
            .context("fetch user me")?
            .into_user()
            .context("missing me user")?;
        eprintln!("user id: {:?}", user.id);

        let followers = client
            .send(&ChannelFollowersRequest::total_only(user.id.clone()))
            .await
            .context("fetch total followers")?
            .total;
        eprintln!("followers: {followers}");

        let mut ws = WebSocket::connect().await?;
        eprintln!("websocket: {:?}", ws.session_id());

        let res = client
            .send(&CreateSubscriptionRequest::new::<ChatMessage>(
                &ChatMessageCondition {
                    broadcaster_user_id: user.id.clone(),
                    user_id: user.id.clone(),
                },
                TransportRequest::WebSocket {
                    session_id: ws.session_id().clone(),
                },
            )?)
            .await
            .context("create subscription")?;
        eprintln!("{res:#?}");

        let res = client
            .send(&CreateSubscriptionRequest::new::<Follow>(
                &FollowCondition {
                    broadcaster_user_id: user.id.clone(),
                    moderator_user_id: user.id.clone(),
                },
                TransportRequest::WebSocket {
                    session_id: ws.session_id().clone(),
                },
            )?)
            .await
            .context("create subscription")?;
        eprintln!("{res:#?}");

        let res = client
            .send(&CreateSubscriptionRequest::new::<StreamOnline>(
                &StreamOnlineCondition {
                    broadcaster_user_id: user.id.clone(),
                },
                TransportRequest::WebSocket {
                    session_id: ws.session_id().clone(),
                },
            )?)
            .await
            .context("create subscription")?;
        eprintln!("{res:#?}");

        let res = client
            .send(&CreateSubscriptionRequest::new::<StreamOffline>(
                &StreamOfflineCondition {
                    broadcaster_user_id: user.id.clone(),
                },
                TransportRequest::WebSocket {
                    session_id: ws.session_id().clone(),
                },
            )?)
            .await
            .context("create subscription")?;
        eprintln!("{res:#?}");

        if let Some(stream) = client
            .send(&StreamsRequest::user_id(user.id.clone()))
            .await
            .context("load stream info")?
            .into_stream()
        {
            let timestamp = stream.started_at.with_timezone(&chrono_tz::Europe::Berlin);
            println!(
                "{} {} {}",
                timestamp.format("%T").to_string().dark_grey(),
                "stream already online".italic().green(),
                stream_info(&stream),
            );
        } else {
            let channel = client
                .send(&ChannelsRequest::id(user.id.clone()))
                .await
                .context("load channel info")?
                .into_channel()
                .context("missing channel")?;
            println!(
                "{} {} {}",
                "--:--:--".dark_grey(),
                "stream is offline".italic().red(),
                channel_info(&channel),
            );
        }

        while let Some((timestamp, notification)) = ws.next().await? {
            if let Some(message) = notification.event::<ChatMessage>()? {
                sound_system.play_sound_for_event(Event::Message);
                // eprintln!("{message:#?}");

                let timestamp = timestamp.with_timezone(&chrono_tz::Europe::Berlin);
                let color = parse_color(&message.color, &message.chatter_user_id);
                println!(
                    "{} {} {}",
                    timestamp.format("%T").to_string().dark_grey(),
                    message.chatter_user_name.with(color).bold(),
                    message.message.text,
                );
            } else if let Some(follow) = notification.event::<Follow>()? {
                sound_system.play_sound_for_event(Event::Follow);
                // eprintln!("{follow:#?}");

                let timestamp = follow.followed_at.with_timezone(&chrono_tz::Europe::Berlin);
                let follower = client
                    .send(&ChatColorsRequest::id(follow.user_id.clone()))
                    .await
                    .context("load chat color for follow message")?
                    .into_chat_color()
                    .context("unable to load char color for follow message")?;
                let color = parse_color(&follower.color, &follow.user_id);
                println!(
                    "{} {} {}",
                    timestamp.format("%T").to_string().dark_grey(),
                    follow.user_name.with(color).bold(),
                    "has followed you".italic(),
                );
            } else if let Some(online) = notification.event::<StreamOnline>()? {
                sound_system.play_sound_for_event(Event::Online);
                // eprintln!("{online:#?}");

                let timestamp = online.started_at.with_timezone(&chrono_tz::Europe::Berlin);
                let stream = client
                    .send(&StreamsRequest::user_id(user.id.clone()))
                    .await
                    .context("load stream info")?
                    .into_stream()
                    .context("missing stream")?;
                println!(
                    "{} {} {}",
                    timestamp.format("%T").to_string().dark_grey(),
                    "stream went online".italic().green(),
                    stream_info(&stream),
                );
            } else if let Some(offline) = notification.event::<StreamOffline>()? {
                sound_system.play_sound_for_event(Event::Offline);
                // eprintln!("{offline:#?}");
                let _ = offline;

                let timestamp = timestamp.with_timezone(&chrono_tz::Europe::Berlin);
                let _ = dbg!(client.send(&StreamsRequest::user_id(user.id.clone())).await);
                let channel = client
                    .send(&ChannelsRequest::id(user.id.clone()))
                    .await
                    .context("load channel info")?
                    .into_channel()
                    .context("missing channel")?;
                println!(
                    "{} {} {}",
                    timestamp.format("%T").to_string().dark_grey(),
                    "stream went offline".italic().red(),
                    channel_info(&channel),
                );
            } else {
                eprintln!("unknown notification event: {notification:#?}");
            }
        }

        Ok(())
    }
}

fn stream_info(stream: &Stream) -> String {
    stream_or_channel_info(
        &stream.title,
        &stream.tags,
        &stream.game_name,
        &stream.language,
    )
}

fn channel_info(channel: &Channel) -> String {
    stream_or_channel_info(
        &channel.title,
        &channel.tags,
        &channel.game_name,
        &channel.broadcaster_language,
    )
}

fn stream_or_channel_info(title: &str, tags: &[String], game_name: &str, language: &str) -> String {
    use std::fmt::Write as _;

    let mut info = String::new();

    let mut append_info = |key: &str, value: &str| {
        write!(info, "\n   {} {}", key.dark_grey(), value).unwrap();
    };

    append_info("Title   ", title);
    append_info("Tags    ", &tags.join(", "));
    append_info("Category", game_name);
    append_info("Language", language);
    info
}

fn parse_color(color: &str, user_id: &str) -> Color {
    try_parse_color(color).unwrap_or_else(|| random_color(user_id))
}

fn try_parse_color(color: &str) -> Option<Color> {
    fn parse_hex(b: u8) -> Option<u8> {
        Some(match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 10,
            b'A'..=b'F' => b - b'A' + 10,
            _ => return None,
        })
    }
    let color = color.strip_prefix('#')?.as_bytes();
    if color.len() != 6 {
        return None;
    }

    let mut iter = color
        .chunks(2)
        .map(|c| Some((parse_hex(c[0])? << 4) | parse_hex(c[1])?));
    let r = iter.next()??;
    let g = iter.next()??;
    let b = iter.next()??;
    Some(Color::Rgb { r, g, b })
}

fn random_color(user_id: &str) -> Color {
    let mut hasher = DefaultHasher::new();
    user_id.hash(&mut hasher);
    let hash = hasher.finish();
    const COLORS: [Color; 14] = [
        Color::DarkGrey,
        Color::Red,
        Color::DarkRed,
        Color::Green,
        Color::DarkGreen,
        Color::Yellow,
        Color::DarkYellow,
        Color::Blue,
        Color::DarkBlue,
        Color::Magenta,
        Color::DarkMagenta,
        Color::Cyan,
        Color::DarkCyan,
        Color::Grey,
    ];
    COLORS[(hash % COLORS.len() as u64) as usize]
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
