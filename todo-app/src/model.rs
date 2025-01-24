use std::{cell::RefCell, fs, ops::ControlFlow, path::PathBuf, time::Duration};

use anyhow::{Context, Result};
use crokey::KeyCombination;
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
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
    todos: Vec<Todo>,

    #[serde(skip)]
    pub path: PathBuf,

    #[serde(skip)]
    pub keybindings: Keybindings,

    #[serde(skip)]
    list_state: RefCell<ListState>,

    #[serde(skip)]
    index: usize,

    #[serde(skip)]
    is_selected: bool,

    #[serde(skip)]
    pub timeout: Option<Duration>,

    #[serde(skip)]
    cursor_y: Option<usize>,
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
        let result = if let Some(cursor_y) = self.cursor_y {
            self.update_insert(event, cursor_y)
        } else {
            self.update_normal(event)
        };

        if self.todos.is_empty() {
            self.todos.push(Todo::default());
            self.reselect();
        }

        self.timeout = if self.is_selected && self.cursor_y.is_none() {
            Some(Duration::from_secs(10))
        } else {
            None
        };

        self.list_state.get_mut().select(Some(self.index));

        result
    }

    fn update_normal(&mut self, event: Option<Event>) -> Result<ControlFlow<()>> {
        let Some(event) = event else {
            return Command::Unselect.run(self);
        };

        match event {
            Event::FocusGained => {}
            Event::FocusLost => {
                return Command::Unselect.run(self);
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

    fn update_insert(
        &mut self,
        event: Option<Event>,
        mut cursor_y: usize,
    ) -> Result<ControlFlow<()>> {
        self.timeout = None;
        let Some(event) = event else {
            return Ok(ControlFlow::Continue(()));
        };

        if !self.is_selected {
            self.cursor_y = None;
            return Ok(ControlFlow::Continue(()));
        }

        let Some(todo) = self.todos.get_mut(self.index) else {
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
                if event.modifiers.difference(KeyModifiers::SHIFT).is_empty() {
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
                            let level = todo.level;
                            self.change_selection(|model| {
                                model.todos.insert(
                                    model.index + 1,
                                    Todo {
                                        level,
                                        ..Default::default()
                                    },
                                );
                                model.index += 1;
                                model.cursor_y = Some(0);
                            });
                        }
                        KeyCode::Char(c) => {
                            todo.text.insert(todo.text.char_to_byte_index(cursor_y), c);
                            self.cursor_y = Some(cursor_y + 1);
                        }
                        _ => {}
                    }
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
            if self.is_selected {
                if let Some(todo) = self.todos.get(self.index) {
                    return Some((
                        u16::try_from(4 + todo.level * 2 + y).unwrap(),
                        u16::try_from(3 + self.index - self.list_state.borrow().offset()).unwrap(),
                    ));
                }
            }
        }
        None
    }

    fn with_selected(&mut self, f: impl FnOnce(&mut Todo)) {
        if self.is_selected {
            if let Some(todo) = self.todos.get_mut(self.index) {
                f(todo);
            }
        }
    }

    fn unselect(&mut self) {
        self.with_selected(|t| t.selected = false);
    }

    fn reselect(&mut self) {
        self.with_selected(|t| t.selected = true);
    }

    fn change_selection<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> Option<T> {
        let val = if self.is_selected {
            self.unselect();
            Some(f(self))
        } else {
            self.is_selected = true;
            None
        };
        self.reselect();
        val
    }

    fn with_selected_or_select<T>(&mut self, f: impl FnOnce(&mut Todo) -> T) -> Option<T> {
        if self.is_selected {
            self.todos.get_mut(self.index).map(f)
        } else {
            self.is_selected = true;
            self.reselect();
            None
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
    Unselect,
    ToggleSelect,
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
            (crokey::key! {esc}, Self::ToggleSelect),
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
        match self {
            Self::Quit => return Ok(ControlFlow::Break(())),
            Self::GoDown => {
                model.change_selection(|model| {
                    model.index += 1;
                    if model.index >= model.todos.len() {
                        model.index = model.todos.len().saturating_sub(1);
                    }
                });
            }
            Self::GoUp => {
                model.change_selection(|model| {
                    model.index = model.index.saturating_sub(1);
                });
            }
            Self::Leave => {
                model.cursor_y = None;
            }
            Self::Unselect => {
                model.unselect();
                model.is_selected = false;
            }
            Self::ToggleSelect => {
                model.unselect();
                model.is_selected ^= true;
                model.reselect();
            }
            Self::Toggle => {
                model.with_selected_or_select(|t| t.state.next());
            }
            Self::Indent => {
                model.with_selected_or_select(|t| t.level_incr());
            }
            Self::Outdent => {
                model.with_selected_or_select(|t| t.level_decr());
            }
            Self::Insert => {
                model.cursor_y = model.with_selected_or_select(|_| 0);
            }
            Self::Append => {
                model.cursor_y = model.with_selected_or_select(|t| t.text.chars().count());
            }
            Self::InsertBelow => {
                if let Some(level) = model.with_selected_or_select(|t| t.level) {
                    model.change_selection(|model| {
                        model.todos.insert(
                            model.index + 1,
                            Todo {
                                level,
                                ..Default::default()
                            },
                        );
                        model.index += 1;
                        model.cursor_y = Some(0);
                    });
                }
            }
            Self::InsertAbove => {
                if let Some(level) = model.with_selected_or_select(|t| t.level) {
                    model.change_selection(|model| {
                        model.todos.insert(
                            model.index,
                            Todo {
                                level,
                                ..Default::default()
                            },
                        );
                        model.cursor_y = Some(0);
                    });
                }
            }
            Self::Delete => {
                model.change_selection(|model| {
                    model.todos.remove(model.index);
                    if model.index >= model.todos.len() {
                        model.index = model.todos.len().saturating_sub(1);
                    }
                });
            }
            Self::Save => {
                model.save()?;
            }
        }

        Ok(ControlFlow::Continue(()))
    }
}
