use std::{cell::RefCell, fs, ops::ControlFlow, path::PathBuf, time::Duration};

use anyhow::{Context, Result};
use crokey::KeyCombination;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Layout},
    style::Stylize,
    text::Text,
    widgets::{List, ListState},
    Frame,
};
use serde::{Deserialize, Serialize};

use crate::{config::Keybindings, todo::Todo, CharToByteIndex};

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Model {
    title: String,

    #[serde(default, rename = "todo")]
    pub todos: Vec<Todo>,

    #[serde(skip)]
    pub path: PathBuf,

    #[serde(skip)]
    pub keybindings: Keybindings,

    #[serde(skip)]
    pub list_state: RefCell<ListState>,

    #[serde(skip)]
    prev_selected: Option<usize>,

    #[serde(skip)]
    pub timeout: Option<Duration>,

    #[serde(skip)]
    pub cursor_y: Option<usize>,
}

impl Model {
    // pub fn map_event(&mut self, event: Option<Event>) -> Option<ChangeEvent>;
    // pub fn apply_change(&mut self, change: ChangeEvent);
    // pub fn change_change(&mut self, change: ChangeEvent);

    pub fn save(&self) -> Result<()> {
        fs::write(
            self.path.as_path(),
            toml::to_string(self).context("serialize data")?,
        )
        .context("write data")
    }

    pub fn update(&mut self, event: Option<Event>) -> Result<ControlFlow<()>> {
        if let Some(cursor_y) = self.cursor_y {
            return self.update_insert(event, cursor_y);
        }

        if let Some(event) = event {
            self.update_normal(event)
        } else {
            self.update_timeout();
            Ok(ControlFlow::Continue(()))
        }
    }

    fn update_normal(&mut self, event: Event) -> Result<ControlFlow<()>> {
        match event {
            Event::FocusGained => {}
            Event::FocusLost => {
                return Command::Leave.run(self);
            }
            Event::Key(event) if event.kind == KeyEventKind::Press => {
                let key: KeyCombination = event.into();
                if let Some(command) = self.keybindings.normal.get(&key).copied() {
                    return command.run(self);
                }
            }
            Event::Key(_) => {}
            Event::Mouse(_) => {}
            Event::Paste(_) => {}
            Event::Resize(_, _) => {}
        }
        Ok(ControlFlow::Continue(()))
    }

    fn update_timeout(&mut self) {
        let list_state = self.list_state.get_mut();

        with_selected(list_state, &mut self.todos, |t| t.selected = false);
        list_state.select(None);
        self.timeout = None;
    }

    fn update_insert(
        &mut self,
        event: Option<Event>,
        mut cursor_y: usize,
    ) -> Result<ControlFlow<()>> {
        self.timeout = None;
        let Some(event) = event else {
            return Ok(ControlFlow::Continue(()));
        };

        let list_state = self.list_state.get_mut();
        let Some(index) = list_state.selected() else {
            self.cursor_y = None;
            return Ok(ControlFlow::Continue(()));
        };
        let Some(todo) = self.todos.get_mut(index) else {
            self.cursor_y = None;
            return Ok(ControlFlow::Continue(()));
        };

        let chars = todo.text.chars().count();
        if cursor_y > chars {
            cursor_y = chars;
            self.cursor_y = Some(cursor_y);
        }

        match event {
            Event::FocusGained => {}
            Event::FocusLost => {}
            Event::Key(event) => {
                if event.kind == KeyEventKind::Press {
                    let key: KeyCombination = event.into();
                    if let Some(command) = self.keybindings.insert.get(&key) {
                        return command.run(self);
                    }
                }
                match event.code {
                    KeyCode::Left => {
                        self.cursor_y = Some(cursor_y.saturating_sub(1));
                    }
                    KeyCode::Right if cursor_y < chars => {
                        self.cursor_y = Some(cursor_y + 1);
                    }
                    KeyCode::Backspace => {
                        let Some(y) = cursor_y.checked_sub(1) else {
                            return Ok(ControlFlow::Continue(()));
                        };
                        todo.text.remove(todo.text.char_to_byte_index(y));
                        self.cursor_y = Some(y);
                    }
                    KeyCode::Delete => {
                        let index = todo.text.char_to_byte_index(cursor_y);
                        if index < todo.text.len() {
                            todo.text.remove(index);
                        }
                    }
                    KeyCode::Enter => {
                        todo.selected = false;
                        if let Some(index) = list_state.selected() {
                            let level = todo.level;
                            self.todos.insert(
                                index + 1,
                                Todo {
                                    level,
                                    ..Default::default()
                                },
                            );
                            list_state.select(Some(index + 1));
                            self.cursor_y = Some(0);
                        }
                        with_selected(list_state, &mut self.todos, |t| t.selected = true);
                    }
                    KeyCode::Char(c) => {
                        todo.text.insert(todo.text.char_to_byte_index(cursor_y), c);
                        self.cursor_y = Some(cursor_y + 1);
                    }
                    _ => {}
                }
            }
            Event::Mouse(_) => {}
            Event::Paste(_) => {}
            Event::Resize(_, _) => {}
        }

        Ok(ControlFlow::Continue(()))
    }

    pub fn draw(&self, frame: &mut Frame) {
        let vertical = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Fill(1),
        ]);
        let [title_area, underline_area, main_area] = vertical.areas(frame.area());

        let text = Text::raw(self.title.as_str()).bold();
        frame.render_widget(text, title_area);

        let text = Text::raw("=".repeat(self.title.len())).bold();
        frame.render_widget(text, underline_area);

        let list = List::new(self.todos.iter().map(Todo::to_text));

        frame.render_stateful_widget(list, main_area, &mut self.list_state.borrow_mut());
    }

    pub fn cursor_position(&mut self) -> Option<(u16, u16)> {
        if let Some(y) = self.cursor_y {
            let list_state = self.list_state.get_mut();
            if let Some(index) = list_state.selected() {
                if let Some(todo) = self.todos.get(index) {
                    return Some((
                        u16::try_from(4 + todo.level * 2 + y).unwrap(),
                        u16::try_from(3 + index - list_state.offset()).unwrap(),
                    ));
                }
            }
        }
        None
    }
}

fn with_selected(list_state: &mut ListState, todos: &mut [Todo], f: impl FnOnce(&mut Todo)) {
    if let Some(index) = list_state.selected() {
        if let Some(todo) = todos.get_mut(index) {
            f(todo);
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Command {
    Quit,
    GoDown,
    GoUp,
    Leave,
    Toggle,
    Indent,
    Outdent,
    Insert,
    Append,
    InsertAbove,
    InsertBelow,
    Delete,
    Save,
}

impl Command {
    pub fn normal_keybindings() -> impl Iterator<Item = (KeyCombination, Self)> {
        [
            (crokey::key! {q}, Self::Quit),
            (crokey::key! {j}, Self::GoDown),
            (crokey::key! {k}, Self::GoUp),
            (crokey::key! {esc}, Self::Leave),
            (crokey::key! {space}, Self::Toggle),
            (crokey::key! {'>'}, Self::Indent),
            (crokey::key! {'<'}, Self::Outdent),
            (crokey::key! {i}, Self::Insert),
            (crokey::key! {a}, Self::Append),
            (crokey::key! {shift-o}, Self::InsertAbove),
            (crokey::key! {o}, Self::InsertBelow),
            (crokey::key! {d}, Self::Delete),
            (crokey::key! {s}, Self::Save),
        ]
        .into_iter()
    }

    pub fn insert_keybindings() -> impl Iterator<Item = (KeyCombination, Self)> {
        [
            (crokey::key! {esc}, Self::Leave),
            (crokey::key! {alt-'>'}, Self::Indent),
            (crokey::key! {alt-'<'}, Self::Outdent),
        ]
        .into_iter()
    }

    fn run(self, model: &mut Model) -> Result<ControlFlow<()>> {
        let list_state = model.list_state.get_mut();
        match self {
            Self::Quit => return Ok(ControlFlow::Break(())),
            Self::GoDown => {
                with_selected(list_state, &mut model.todos, |t| t.selected = false);
                if list_state.selected().is_none() {
                    list_state.select(model.prev_selected);
                }
                list_state.select_next();
                with_selected(list_state, &mut model.todos, |t| t.selected = true);
            }
            Self::GoUp => {
                with_selected(list_state, &mut model.todos, |t| t.selected = false);
                if list_state.selected().is_none() {
                    list_state.select(model.prev_selected);
                }
                list_state.select_previous();
                with_selected(list_state, &mut model.todos, |t| t.selected = true);
            }
            Self::Leave if model.cursor_y.is_some() => {
                model.cursor_y = None;
            }
            Self::Leave => {
                with_selected(list_state, &mut model.todos, |t| t.selected = false);
                if list_state.selected().is_none() {
                    list_state.select(model.prev_selected);
                } else {
                    list_state.select(None);
                }
                with_selected(list_state, &mut model.todos, |t| t.selected = true);
            }
            Self::Toggle => {
                with_selected(list_state, &mut model.todos, |t| t.state.next());
                if list_state.selected().is_none() {
                    list_state.select(model.prev_selected);
                    with_selected(list_state, &mut model.todos, |t| t.selected = true);
                }
            }
            Self::Indent => {
                with_selected(list_state, &mut model.todos, |t| t.level_incr());
                if list_state.selected().is_none() {
                    list_state.select(model.prev_selected);
                    with_selected(list_state, &mut model.todos, |t| t.selected = true);
                }
            }
            Self::Outdent => {
                with_selected(list_state, &mut model.todos, |t| {
                    t.level_decr();
                });
                if list_state.selected().is_none() {
                    list_state.select(model.prev_selected);
                    with_selected(list_state, &mut model.todos, |t| t.selected = true);
                }
            }
            Self::Insert => {
                model.cursor_y = Some(0);
            }
            Self::Append => {
                if let Some(index) = list_state.selected() {
                    if let Some(todo) = model.todos.get(index) {
                        model.cursor_y = Some(todo.text.chars().count());
                    }
                }
            }
            Self::InsertBelow => {
                with_selected(list_state, &mut model.todos, |t| t.selected = false);
                if let Some(index) = list_state.selected() {
                    if let Some(todo) = model.todos.get(index) {
                        model.todos.insert(
                            index + 1,
                            Todo {
                                level: todo.level,
                                ..Default::default()
                            },
                        );
                        list_state.select(Some(index + 1));
                        model.cursor_y = Some(0);
                    }
                }
                with_selected(list_state, &mut model.todos, |t| t.selected = true);
            }
            Self::InsertAbove => {
                with_selected(list_state, &mut model.todos, |t| t.selected = false);
                if let Some(index) = list_state.selected() {
                    if let Some(todo) = model.todos.get(index) {
                        model.todos.insert(
                            index,
                            Todo {
                                level: todo.level,
                                ..Default::default()
                            },
                        );
                        model.cursor_y = Some(0);
                    }
                }
                with_selected(list_state, &mut model.todos, |t| t.selected = true);
            }
            Self::Delete => {
                with_selected(list_state, &mut model.todos, |t| t.selected = false);
                if let Some(index) = list_state.selected() {
                    if index < model.todos.len() {
                        model.todos.remove(index);
                    }
                }
                with_selected(list_state, &mut model.todos, |t| t.selected = true);
            }
            Self::Save => {
                model.save()?;
            }
        }

        if let Some(index) = model.list_state.get_mut().selected() {
            model.prev_selected = Some(index);
            model.timeout = Some(Duration::from_secs(10));
        }

        Ok(ControlFlow::Continue(()))
    }
}
