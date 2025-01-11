use std::{
    collections::HashMap,
    fmt::Write as _,
    hash::{DefaultHasher, Hash, Hasher},
    io, iter,
};

use anyhow::{Context, Error, Result};
use chrono::{DateTime, Utc};
use clap::Parser;
use crossterm::style::{Color, Stylize};
use tokio::{sync::mpsc, task::LocalSet};
use twitch_api::{
    auth::{self, Scope},
    channel::{Channel, ChannelsRequest},
    chat::{ChatColorsRequest, SendChatMessageRequest},
    client::Client,
    events::{
        chat::{ChatMessage, ChatMessageCondition},
        follow::{Follow, FollowCondition},
        stream::{StreamOffline, StreamOfflineCondition, StreamOnline, StreamOnlineCondition},
        subscription::{
            CreateSubscriptionRequest, DeleteSubscriptionRequest, GetSubscriptionsRequest,
            TransportRequest,
        },
        ws::{NotificationMessage, WebSocket},
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

        let (sender, mut receiver) = mpsc::unbounded_channel();

        {
            let sender = sender.clone();
            tokio::task::spawn_local(async move {
                loop {
                    let item = match ws.next().await {
                        Ok(None) => break,
                        Ok(Some((timestamp, notification))) => Item::Notification {
                            timestamp,
                            notification,
                        },
                        Err(err) => Item::WebSocketError(err),
                    };
                    if sender.send(item).is_err() {
                        break;
                    }
                }
            });
        }

        tokio::task::spawn_blocking(move || {
            for line in io::stdin().lines() {
                let item = match line {
                    Ok(message) => {
                        let _ = crossterm::execute!(
                            io::stdout(),
                            crossterm::cursor::MoveUp(1),
                            crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine)
                        );
                        if message.is_empty() {
                            continue;
                        }
                        Item::SendMessage { message }
                    }
                    Err(err) => Item::StdinError(err),
                };
                if sender.send(item).is_err() {
                    break;
                }
            }
        });

        let mut poll: Option<Poll> = None;

        while let Some(item) = receiver.recv().await {
            match item {
                Item::Notification {
                    timestamp,
                    notification,
                } => {
                    if let Some(message) = notification.event::<ChatMessage>()? {
                        sound_system.play_sound_for_event(Event::Message);
                        // eprintln!("{message:#?}");

                        if let Some(poll) = &mut poll {
                            poll.vote(&message.chatter_user_id, &message.message.text);
                        }

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

                        let timestamp =
                            follow.followed_at.with_timezone(&chrono_tz::Europe::Berlin);
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
                Item::SendMessage { message } => {
                    let message = if let Some(message) = message.strip_prefix('#') {
                        let (cmd, text) = message.split_once(' ').unwrap_or((message, ""));
                        match (cmd, text) {
                            ("poll", _) => {
                                if poll.is_some() {
                                    eprintln!("poll already active, try #end poll");
                                    continue;
                                }

                                let mut message = "Frage:".to_string();
                                let mut options = Vec::new();
                                for (i, option) in text.split(',').enumerate() {
                                    if i != 0 {
                                        message.push_str(" -");
                                    }
                                    let option = option.trim();
                                    options.push(option.into());
                                    write!(message, " {i}={option}").unwrap();
                                }
                                poll = Some(Poll {
                                    options,
                                    votes: Default::default(),
                                });
                                message
                            }
                            ("end", "poll") => {
                                let Some(poll) = poll.take() else {
                                    eprintln!("no active poll");
                                    continue;
                                };
                                poll.result()
                            }
                            _ => {
                                eprintln!("unknown command: #{cmd} {text:?}");
                                continue;
                            }
                        }
                    } else {
                        message
                    };
                    let message = client
                        .send(&SendChatMessageRequest {
                            broadcaster_id: user.id.clone(),
                            sender_id: user.id.clone(),
                            message,
                            reply_parent_message_id: None,
                        })
                        .await
                        .context("send message")?
                        .into_chat_message()
                        .context("missing chat message")?;
                    if !message.is_sent {
                        eprintln!("{message:#?}");
                    }
                }
                Item::WebSocketError(err) => return Err(err).context("receive next notification"),
                Item::StdinError(err) => return Err(err).context("read message from stdin"),
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

enum Item {
    Notification {
        timestamp: DateTime<Utc>,
        notification: NotificationMessage,
    },
    SendMessage {
        message: String,
    },
    WebSocketError(Error),
    StdinError(io::Error),
}

struct Poll {
    options: Vec<String>,
    votes: HashMap<String, usize>,
}
impl Poll {
    fn vote(&mut self, user_id: &str, text: &str) {
        let Ok(n) = text.split(' ').next().unwrap().parse() else {
            return;
        };
        self.votes.insert(user_id.into(), n);
    }

    fn result(self) -> String {
        let mut votes = vec![0; self.options.len()];
        for vote in self.votes.into_values() {
            votes[vote] += 1;
        }
        let max = votes.iter().copied().max().unwrap_or(0);
        if max == 0 {
            "Ergebnis: Keine Stimmen".into()
        } else {
            let mut message = format!("Ergebnis[{max}]:");
            let mut first = true;
            for (option, votes) in iter::zip(self.options, votes) {
                if votes == max {
                    if first {
                        first = false;
                    } else {
                        message.push_str(" -");
                    }
                    write!(message, " {option}").unwrap();
                }
            }
            message
        }
    }
}
