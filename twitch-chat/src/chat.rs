use std::{
    collections::HashMap,
    fmt::Write,
    hash::{DefaultHasher, Hash, Hasher},
    iter,
    num::NonZeroUsize,
    ops::ControlFlow,
    pin::pin,
    sync::LazyLock,
};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use crokey::KeyCombination;
use crossterm::event::{
    Event as InputEvent, EventStream, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind,
};
use futures::{
    StreamExt,
    future::{self, Either},
};
use nucleo::{Config, Utf32String};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget, Wrap},
};
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::mpsc;
use twitch_api::{
    channel::{Channel, ChannelsRequest},
    chat::{ChatAnnouncementColor, SendChatAnnouncementRequest, SendChatMessageRequest},
    client::AuthenticatedClient,
    events::{
        chat::{
            ChatMessageFragment, ChatMessageMessage, message::ChatMessage,
            notification::ChatNotification,
        },
        follow::Follow,
        stream::{StreamOffline, StreamOnline},
        ws::{NotificationMessage, WebSocket},
    },
    stream::{Stream, StreamsRequest},
    user::User,
};

use crate::{
    config::{Event as SoundEvent, Keybindings},
    sound_system::SoundSystem,
    store::{Event, Store},
};

pub async fn run(
    mut terminal: DefaultTerminal,
    keybindings: Keybindings,
    store: Store,
    client: &mut AuthenticatedClient,
    user: User,
    mut ws: WebSocket,
    sound_system: SoundSystem,
) -> Result<()> {
    let mut state = State {
        keybindings,
        store,
        client,
        user,
        sound_system,
        offset: None,
        focus: FocusState::None,
        search: String::new(),
        message: String::new(),
        error: String::new(),
        poll: None,
    };

    state.store.push(Event::Started {
        started_at: Utc::now(),
    })?;

    let (sender, mut receiver) = mpsc::unbounded_channel();
    tokio::task::spawn_local(async move {
        while let Some(notification) = ws.next().await.transpose() {
            if sender.send(notification).is_err() {
                break;
            }
        }
    });

    let mut events = EventStream::new();
    let mut events_next = events.next();

    loop {
        state.store.tick();

        terminal
            .draw(|frame| state.draw(frame))
            .context("draw frame")?;

        match future::select(
            events_next,
            future::select(pin!(receiver.recv()), pin!(state.store.search_changed())),
        )
        .await
        {
            Either::Left((event, _)) => {
                let event = event.unwrap().context("read input event")?;
                if state.update(event).await?.is_break() {
                    break Ok(());
                }
                events_next = events.next();
            }
            Either::Right((inner, fut)) => {
                match inner {
                    Either::Left((notification, _)) => {
                        let (timestamp, notification) =
                            notification.context("unreachable: web socket connection closed")??;
                        state.handle(timestamp, notification).await?;
                    }
                    Either::Right(((), _)) => {
                        // nothing to do, tick is called anyway
                    }
                }
                events_next = fut;
            }
        }
    }
}

struct State<'a> {
    keybindings: Keybindings,
    store: Store,
    client: &'a mut AuthenticatedClient,
    user: User,
    sound_system: SoundSystem,
    offset: Option<NonZeroUsize>,
    focus: FocusState,
    search: String,
    message: String,
    error: String,
    poll: Option<Poll>,
}

impl State<'_> {
    fn draw(&mut self, frame: &mut Frame) {
        let mut area = frame.area();

        if !self.message.is_empty() || self.focus.is_message() {
            let message_area;
            (area, message_area) = bottom_area(area, 1);
            let widget =
                Line::from_iter([Span::raw("Message: ").dark_gray(), Span::raw(&self.message)]);
            frame.render_widget(widget, message_area);

            let block_area;
            (area, block_area) = bottom_area(area, 1);
            let block = Block::new().borders(Borders::TOP).dark_gray();
            frame.render_widget(block, block_area);

            if let FocusState::Message(offset) = self.focus {
                frame.set_cursor_position((9 + u16::try_from(offset).unwrap(), message_area.y));
            }
        }

        if !self.error.is_empty() {
            let error = Paragraph::new(self.error.as_str())
                .red()
                .wrap(Wrap { trim: false });
            let height = error.line_count(area.width);

            let error_area;
            (area, error_area) = bottom_area(area, height);
            frame.render_widget(error, error_area);

            let block_area;
            (area, block_area) = bottom_area(area, 1);
            let block = Block::new().borders(Borders::TOP).dark_gray();
            frame.render_widget(block, block_area);
        }

        if !self.search.is_empty() || self.focus.is_search() {
            let search_area;
            (area, search_area) = bottom_area(area, 1);
            let widget =
                Line::from_iter([Span::raw("Search: ").dark_gray(), Span::raw(&self.search)]);
            frame.render_widget(widget, search_area);

            let block_area;
            (area, block_area) = bottom_area(area, 1);
            let block = Block::new().borders(Borders::TOP).dark_gray();
            frame.render_widget(block, block_area);

            if let FocusState::Search(offset) = self.focus {
                frame.set_cursor_position((8 + u16::try_from(offset).unwrap(), search_area.y));
            }
        }

        let events = self.store.events(&mut self.offset);
        for event in events {
            frame.render_stateful_widget(event, area, &mut area);
            if area.height == 0 {
                break;
            }
        }
    }

    fn keybinding(&self, key: KeyCombination) -> Option<Command> {
        let keybindings = if self.focus.is_none() {
            &self.keybindings.normal
        } else {
            &self.keybindings.insert
        };
        keybindings.get(&key).copied()
    }

    async fn update(&mut self, event: InputEvent) -> Result<ControlFlow<()>> {
        match event {
            InputEvent::FocusGained => {}
            InputEvent::FocusLost => {}
            InputEvent::Key(event) if event.kind == KeyEventKind::Press => {
                if let Some(command) = self.keybinding(event.into()) {
                    return self.run(command);
                }

                if event.modifiers.difference(KeyModifiers::SHIFT).is_empty() {
                    let (text, offset) = match &mut self.focus {
                        FocusState::None => return Ok(ControlFlow::Continue(())),
                        FocusState::Message(offset) => (&mut self.message, offset),
                        FocusState::Search(offset) => (&mut self.search, offset),
                    };
                    match event.code {
                        KeyCode::Enter => {
                            self.error = String::new();
                            match self.focus {
                                FocusState::None => {}
                                FocusState::Message(_) => {
                                    self.send_message().await?;
                                }
                                FocusState::Search(_) => {
                                    self.focus = FocusState::None;
                                }
                            }
                        }
                        KeyCode::Backspace if *offset > 0 => {
                            *offset -= 1;
                            text.remove(text.char_to_byte_index(*offset));
                        }
                        KeyCode::Delete => {
                            let index = text.char_to_byte_index(*offset);
                            if index < text.len() {
                                text.remove(index);
                            }
                        }
                        KeyCode::Left => {
                            *offset = offset.saturating_sub(1);
                        }
                        KeyCode::Right if *offset < text.chars().count() => {
                            *offset += 1;
                        }
                        KeyCode::Char(c) => {
                            text.insert(text.char_to_byte_index(*offset), c);
                            *offset += 1;
                        }
                        KeyCode::Tab if self.focus.is_message() => {
                            self.autocomplete();
                        }
                        _ => {}
                    }
                    if self.focus.is_search() {
                        self.do_search();
                    }
                }
            }
            InputEvent::Key(_) => {}
            InputEvent::Mouse(event) => match event.kind {
                MouseEventKind::Down(_button) => {}
                MouseEventKind::Up(_button) => {}
                MouseEventKind::Drag(_button) => {}
                MouseEventKind::Moved => {}
                MouseEventKind::ScrollDown => return self.run(Command::GoDown),
                MouseEventKind::ScrollUp => return self.run(Command::GoUp),
                MouseEventKind::ScrollLeft => {}
                MouseEventKind::ScrollRight => {}
            },
            InputEvent::Paste(_) => {}
            InputEvent::Resize(_, _) => {}
        }
        Ok(ControlFlow::Continue(()))
    }

    fn run(&mut self, command: Command) -> Result<ControlFlow<()>> {
        match command {
            Command::Quit => return Ok(ControlFlow::Break(())),
            Command::Leave => {
                if !self.focus.is_none() {
                    self.focus = FocusState::None;
                    self.error = String::new();
                } else if self.offset.is_some() {
                    self.offset = None;
                } else if !self.message.is_empty() {
                    self.message = String::new();
                } else if !self.search.is_empty() {
                    self.search = String::new();
                    self.do_search();
                }
            }
            Command::GoUp => {
                self.offset = NonZeroUsize::new({
                    if let Some(offset) = self.offset {
                        offset.get()
                    } else {
                        self.store.events_len()
                    }
                    .saturating_sub(1)
                })
                .or_else(|| NonZeroUsize::new(1))
            }
            Command::GoDown => {
                if let Some(offset) = self.offset {
                    let offset = offset.get() + 1;
                    self.offset = if offset < self.store.events_len() {
                        NonZeroUsize::new(offset)
                    } else {
                        None
                    };
                }
            }
            Command::Search => {
                self.focus = FocusState::Search(0);
            }
            Command::Message => {
                self.focus = FocusState::Message(0);
            }
        }
        Ok(ControlFlow::Continue(()))
    }

    async fn send_message(&mut self) -> Result<()> {
        let message = if let Some(message) = self.message.strip_prefix('/') {
            let (cmd, text) = message.split_once(' ').unwrap_or((message, ""));
            match (cmd, text) {
                ("poll", _) => {
                    if self.poll.is_some() {
                        self.error = "poll already active, try #end poll".into();
                        return Ok(());
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
                    self.poll = Some(Poll {
                        options,
                        votes: Default::default(),
                    });
                    message
                }
                ("end", "poll") => {
                    let Some(poll) = self.poll.take() else {
                        self.error = "no active poll".into();
                        return Ok(());
                    };
                    poll.result()
                }
                ("announce", _) if !text.is_empty() => {
                    self.client
                        .send(&SendChatAnnouncementRequest {
                            broadcaster_id: self.user.id.clone(),
                            moderator_id: self.user.id.clone(),
                            message: text.into(),
                            color: ChatAnnouncementColor::Primary,
                        })
                        .await
                        .context("send chat announcement")?;
                    self.clear_message();
                    return Ok(());
                }
                ("pin", _) if !text.is_empty() => {
                    self.error = "/pin not yet exposed by the twitch API".into();
                    self.clear_message();
                    return Ok(());
                }
                ("unpin", "") => {
                    self.error = "/unpin not yet exposed by the twitch API".into();
                    self.clear_message();
                    return Ok(());
                }
                _ => {
                    self.error = format!("unknown command: /{cmd} {text:?}");
                    return Ok(());
                }
            }
        } else {
            self.message.clone()
        };
        let message = self
            .client
            .send(&SendChatMessageRequest {
                broadcaster_id: self.user.id.clone(),
                sender_id: self.user.id.clone(),
                message,
                reply_parent_message_id: None,
            })
            .await
            .context("send message")?
            .into_chat_message()
            .context("missing chat message")?;
        if message.is_sent {
            self.clear_message();
        } else {
            self.error = if let Some(drop_reason) = message.drop_reason {
                format!(
                    "failed to send message ({}): {}",
                    drop_reason.code, drop_reason.message
                )
            } else {
                "failed to send message: no drop reason".into()
            };
        }
        Ok(())
    }

    fn clear_message(&mut self) {
        self.message = String::new();
        self.focus = FocusState::None;
    }

    async fn handle(
        &mut self,
        timestamp: DateTime<Utc>,
        notification: NotificationMessage,
    ) -> Result<()> {
        let extra = if let Some(message) = notification.event::<ChatMessage>()? {
            self.sound_system.play_sound_for_event(SoundEvent::Message);

            if let Some(poll) = &mut self.poll {
                poll.vote(&message.chatter_user_id, &message.message.text);
            }

            Value::Null
        } else if let Some(_notification) = notification.event::<ChatNotification>()? {
            self.sound_system.play_sound_for_event(SoundEvent::Message);
            Value::Null
        } else if let Some(_follow) = notification.event::<Follow>()? {
            self.sound_system.play_sound_for_event(SoundEvent::Follow);
            Value::Null
        } else if let Some(online) = notification.event::<StreamOnline>()? {
            self.sound_system.play_sound_for_event(SoundEvent::Online);

            let stream = self
                .client
                .send(&StreamsRequest::user_id(online.broadcaster_user_id))
                .await
                .context("load stream info")?
                .into_stream()
                .context("missing stream")?;

            serde_json::to_value(stream).context("convert stream info to value")?
        } else if let Some(offline) = notification.event::<StreamOffline>()? {
            self.sound_system.play_sound_for_event(SoundEvent::Offline);

            let channel = self
                .client
                .send(&ChannelsRequest::id(offline.broadcaster_user_id))
                .await
                .context("load channel info")?
                .into_channel()
                .context("missing channel")?;

            serde_json::to_value(channel).context("convert channel info to value")?
        } else {
            Value::Null
        };
        self.store.push(Event::Notification {
            timestamp,
            event: notification.into_event(),
            extra,
        })
    }

    fn do_search(&mut self) {
        self.store.start_search(&self.search);
    }

    fn autocomplete(&mut self) {
        let index = {
            let FocusState::Message(offset) = self.focus else {
                return;
            };
            self.message.char_to_byte_index(offset)
        };

        let message = &self.message[..index];
        if message.starts_with('/') && !message.contains(char::is_whitespace) {
            let mut matcher = nucleo::Matcher::new(Config::DEFAULT);
            let needle: Utf32String = message[1..].into();
            if needle.is_empty() {
                return;
            }

            static HAYSTACKS: LazyLock<Vec<Utf32String>> = LazyLock::new(|| {
                ["poll", "end poll", "announce"]
                    .into_iter()
                    .map(|s| s.into())
                    .collect()
            });

            let max_match = HAYSTACKS
                .iter()
                .filter_map(|haystack| {
                    matcher
                        .fuzzy_match(haystack.slice(..), needle.slice(..))
                        .map(|s| (s, haystack))
                })
                .max();

            if let Some((_score, match_)) = max_match {
                self.message = format!("/{match_} {}", &self.message[index..]);
                self.focus = FocusState::Message(match_.len() + 2);
            }

            return;
        }

        let word = message.split_whitespace().next_back().unwrap();
        if let Some(_needle) = word.strip_prefix('@') {
            // TODO: complete user name
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum FocusState {
    None,
    Message(usize),
    Search(usize),
}

impl FocusState {
    fn is_none(self) -> bool {
        matches!(self, Self::None)
    }

    fn is_message(self) -> bool {
        matches!(self, Self::Message(_))
    }

    fn is_search(self) -> bool {
        matches!(self, Self::Search(_))
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename = "snake_case")]
pub enum Command {
    Quit,
    Leave,
    GoUp,
    GoDown,
    Search,
    Message,
}

impl Command {
    pub fn normal_keybindings() -> impl Iterator<Item = (KeyCombination, Self)> {
        [
            (crokey::key! {q}, Self::Quit),
            (crokey::key! {esc}, Self::Leave),
            (crokey::key! {k}, Self::GoUp),
            (crokey::key! {j}, Self::GoDown),
            (crokey::key! {'/'}, Self::Search),
            (crokey::key! {o}, Self::Message),
        ]
        .into_iter()
    }

    pub fn insert_keybindings() -> impl Iterator<Item = (KeyCombination, Self)> {
        [
            //
            (crokey::key! {esc}, Self::Leave),
            (crokey::key! {up}, Self::GoUp),
            (crokey::key! {down}, Self::GoDown),
        ]
        .into_iter()
    }
}

impl StatefulWidget for &Event {
    type State = Rect;

    fn render(self, mut area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let paragraph = Paragraph::new(self.to_text().unwrap_or_else(|err| {
            Line::from_iter([
                Span::raw("Error: ").bold().red(),
                Span::raw(format!("{err}")).red(),
            ])
            .into()
        }))
        .wrap(Wrap { trim: false });
        let height = paragraph.line_count(area.width);
        (*state, area) = bottom_area(area, height);
        paragraph.render(area, buf)
    }
}

fn bottom_area(area: Rect, height: usize) -> (Rect, Rect) {
    let height = height.min(area.height as usize) as u16;
    let layout = Layout::vertical([Constraint::Fill(1), Constraint::Length(height)]);
    let [remaining, area] = layout.areas(area);
    (remaining, area)
}

impl Event {
    fn to_text(&self) -> Result<Text> {
        Ok(match self {
            Self::Started { started_at } => {
                Line::from_iter([started_at.to_span(), "chat started".italic()])
            }
            Self::Message {
                sent_at,
                user_login,
                text,
            } => Line::from_iter([
                sent_at.to_span(),
                Span::raw(user_login).bold().red(),
                Span::raw(" "),
                Span::raw(text),
            ]),
            Self::Notification {
                timestamp,
                event,
                extra,
            } => {
                let notification = event;
                let mut spans = Vec::new();
                let mut lines = Vec::new();
                if let Some(message) = notification.parse::<ChatMessage>()? {
                    let color = parse_color(&message.color, &message.chatter_user_id);
                    spans.extend([
                        timestamp.to_span(),
                        Span::raw(message.chatter_user_name).bold().fg(color),
                        Span::raw(" "),
                    ]);
                    message_to_spans(&message.message, &mut spans);
                    spans.into()
                } else if let Some(notification) = notification.parse::<ChatNotification>()? {
                    let color = parse_color(&notification.color, &notification.chatter_user_id);
                    spans.extend([
                        timestamp.to_span(),
                        Span::raw(notification.chatter_user_name).bold().fg(color),
                        Span::raw(" "),
                    ]);
                    if !notification.system_message.is_empty() {
                        spans.extend([
                            Span::raw(notification.system_message).italic(),
                            Span::raw(" "),
                        ]);
                    }
                    message_to_spans(&notification.message, &mut spans);
                    spans.into()
                } else if let Some(follow) = notification.parse::<Follow>()? {
                    let follower_color = "";
                    let color = parse_color(follower_color, &follow.user_id);
                    Line::from_iter([
                        follow.followed_at.to_span(),
                        Span::raw(follow.user_name).bold().fg(color),
                        Span::raw(" has followed you").italic(),
                    ])
                } else if let Some(online) = notification.parse::<StreamOnline>()? {
                    let stream: Stream =
                        serde_json::from_value(extra.clone()).context("parse stream info")?;

                    lines.push(Line::from_iter([
                        online.started_at.to_span(),
                        Span::raw("stream went online").italic().green(),
                    ]));
                    stream_info(&stream, &mut lines);
                    return Ok(lines.into());
                } else if let Some(offline) = notification.parse::<StreamOffline>()? {
                    let _ = offline;

                    let channel: Channel =
                        serde_json::from_value(extra.clone()).context("parse channel info")?;

                    lines.push(Line::from_iter([
                        timestamp.to_span(),
                        Span::raw("stream went offline").italic().red(),
                    ]));
                    channel_info(&channel, &mut lines);
                    return Ok(lines.into());
                } else {
                    Line::from_iter([
                        timestamp.to_span(),
                        Span::raw(format!("unknown notification event: {notification:?}")).italic(),
                    ])
                }
            }
        }
        .into())
    }
}

trait ToSpan {
    fn to_span(&self) -> Span<'static>;
}

impl ToSpan for DateTime<Utc> {
    fn to_span(&self) -> Span<'static> {
        Span::raw(
            self.with_timezone(crate::timezone())
                .format("%T ")
                .to_string(),
        )
        .italic()
        .dark_gray()
    }
}

trait CharToByteIndex {
    fn char_to_byte_index(&self, index: usize) -> usize;
}

impl CharToByteIndex for &str {
    fn char_to_byte_index(&self, index: usize) -> usize {
        self.char_indices()
            .nth(index)
            .unwrap_or((self.len(), '\0'))
            .0
    }
}
impl CharToByteIndex for String {
    fn char_to_byte_index(&self, index: usize) -> usize {
        self.as_str().char_to_byte_index(index)
    }
}

fn stream_info(stream: &Stream, lines: &mut Vec<Line>) {
    stream_or_channel_info(
        &stream.title,
        &stream.tags,
        &stream.game_name,
        &stream.language,
        lines,
    )
}

fn channel_info(channel: &Channel, lines: &mut Vec<Line>) {
    stream_or_channel_info(
        &channel.title,
        &channel.tags,
        &channel.game_name,
        &channel.broadcaster_language,
        lines,
    )
}

fn stream_or_channel_info(
    title: &str,
    tags: &[String],
    game_name: &str,
    language: &str,
    lines: &mut Vec<Line>,
) {
    let mut append_info = |key: &'static str, value: String| {
        lines.push(Line::from_iter([
            Span::raw("   "),
            Span::raw(key).dark_gray(),
            Span::raw(value),
        ]));
    };

    append_info("Title    ", title.into());
    append_info("Tags     ", tags.join(", "));
    append_info("Category ", game_name.into());
    append_info("Language ", language.into());
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
    Some(Color::Rgb(r, g, b))
}

fn random_color(user_id: &str) -> Color {
    let mut hasher = DefaultHasher::new();
    user_id.hash(&mut hasher);
    let hash = hasher.finish();
    const COLORS: [Color; 14] = [
        Color::Red,
        Color::Green,
        Color::Yellow,
        Color::Blue,
        Color::Magenta,
        Color::Cyan,
        Color::Gray,
        Color::DarkGray,
        Color::LightRed,
        Color::LightGreen,
        Color::LightYellow,
        Color::LightBlue,
        Color::LightMagenta,
        Color::LightCyan,
    ];
    COLORS[(hash % COLORS.len() as u64) as usize]
}

fn message_to_spans(message: &ChatMessageMessage, spans: &mut Vec<Span>) {
    if message.fragments.is_empty() {
        spans.push(Span::raw("empty chat message").italic().dark_gray());
    }

    for fragment in &message.fragments {
        spans.push(match fragment {
            ChatMessageFragment::Text { text } => Span::raw(text.clone()),
            ChatMessageFragment::Cheermote { text, cheermote: _ } => {
                Span::raw(text.clone()).dark_gray()
            }
            ChatMessageFragment::Emote { text, emote: _ } => Span::raw(text.clone()).dark_gray(),
            ChatMessageFragment::Mention { text, mention: _ } => {
                Span::raw(text.clone()).dark_gray()
            }
        });
    }
}

// impl fmt::Display for Print<&ChatNotificationType> {
//     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//         match self.0 {
//             ChatNotificationType::Sub { .. } => "sub",
//             ChatNotificationType::Resub { .. } => "resub",
//             ChatNotificationType::SubGift { .. } => "sub_gift",
//             ChatNotificationType::CommunitySubGift { .. } => "community_sub_gift",
//             ChatNotificationType::GiftPaidUpgrade { .. } => "gift_paid_upgrade",
//             ChatNotificationType::PrimePaidUpgrade { .. } => "prime_paid_upgrade",
//             ChatNotificationType::Raid { .. } => "raid",
//             ChatNotificationType::Unraid { .. } => "unraid",
//             ChatNotificationType::PayItForward { .. } => "pay_it_forward",
//             ChatNotificationType::Announcement { announcement } => {
//                 return "announcement"
//                     .italic()
//                     .with(match announcement.color {
//                         ChatAnnouncementColor::Blue => Color::Blue,
//                         ChatAnnouncementColor::Green => Color::Green,
//                         ChatAnnouncementColor::Orange => Color::DarkYellow,
//                         ChatAnnouncementColor::Purple => Color::Magenta,
//                         ChatAnnouncementColor::Primary => Color::DarkGrey,
//                     })
//                     .fmt(f);
//             }
//             ChatNotificationType::BitsBadgeTier { .. } => "bits_badge_tier",
//             ChatNotificationType::CharityDonation { .. } => "charity_donation",
//             ChatNotificationType::SharedChatSub { .. } => "shared_chat_sub",
//             ChatNotificationType::SharedChatResub { .. } => "shared_chat_resub",
//             ChatNotificationType::SharedChatSubGift { .. } => "shared_chat_sub_gift",
//             ChatNotificationType::SharedChatCommunitySubGift { .. } => {
//                 "shared_chat_community_sub_gift"
//             }
//             ChatNotificationType::SharedChatGiftPaidUpgrade { .. } => {
//                 "shared_chat_gift_paid_upgrade"
//             }
//             ChatNotificationType::SharedChatPrimePaidUpgrade { .. } => {
//                 "shared_chat_prime_paid_upgrade"
//             }
//             ChatNotificationType::SharedChatRaid { .. } => "shared_chat_raid",
//             ChatNotificationType::SharedChatPayItForward { .. } => "shared_chat_pay_it_forward",
//             ChatNotificationType::SharedChatAnnouncement { .. } => "shared_chat_announcement",
//         }
//         .italic()
//         .dark_grey()
//         .fmt(f)
//     }
// }

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
