use std::{
    cell::RefCell, collections::VecDeque, fs, mem, ops::ControlFlow, path::PathBuf, time::Duration,
};

use anyhow::{Context, Result};
use crokey::KeyCombination;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout},
    style::Stylize,
    text::Text,
    widgets::{List, ListState},
    Frame,
};
use serde::{Deserialize, Serialize};

use crate::{
    config::Keybindings,
    todo::{State, Todo},
    CharToByteIndex,
};

pub fn default_undo_steps() -> usize {
    4096
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Model {
    #[serde(default)]
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
    edit_title: bool,

    #[serde(skip)]
    pub timeout: Option<Duration>,

    #[serde(skip)]
    cursor_y: Option<usize>,

    #[serde(skip)]
    pub max_undo: usize,

    #[serde(skip)]
    undo_buffer: VecDeque<UndoAction>,

    #[serde(skip)]
    redo_buffer: Vec<UndoAction>,
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

    pub fn did_load(&mut self) {
        if self.title.is_empty() {
            self.edit_title = true;
            self.cursor_y = Some(0);
        }

        if self.todos.is_empty() {
            self.todos.push(Todo::default());
            self.reselect();
        }
    }

    fn push_undo(&mut self, action: UndoAction) {
        self.redo_buffer = Vec::new();
        if self.undo_buffer.len() >= self.max_undo {
            self.undo_buffer.pop_front();
        }
        self.undo_buffer.push_back(action);
    }

    fn push_undo_delete(&mut self) {
        self.push_undo(UndoAction::Delete { index: self.index });
    }

    pub fn update(&mut self, event: Option<Event>) -> Result<ControlFlow<()>> {
        let result = if let Some(cursor_y) = self.cursor_y {
            if self.edit_title {
                self.update_insert_title(event, cursor_y)
            } else {
                self.update_insert(event, cursor_y)
            }
        } else {
            self.update_normal(event)
        };

        if self.todos.is_empty() {
            self.todos.push(Todo::default());
            self.push_undo_delete();
            self.reselect();
        }

        if self.cursor_y.is_none() {
            self.edit_title = false;
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

                match Self::update_text(cursor_y, &mut todo.text, chars, event) {
                    None => {}
                    Some(None) => {
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
                        self.push_undo_delete();
                    }
                    Some(Some(y)) => {
                        self.cursor_y = Some(y);
                    }
                }
            }
            Event::Mouse(_) => {}
            Event::Paste(_) => {}
            Event::Resize(_, _) => {}
        }

        Ok(ControlFlow::Continue(()))
    }

    fn update_insert_title(
        &mut self,
        event: Option<Event>,
        mut cursor_y: usize,
    ) -> Result<ControlFlow<()>> {
        self.timeout = None;
        let Some(event) = event else {
            return Ok(ControlFlow::Continue(()));
        };

        let chars = self.title.chars().count();
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
                if let Some(Some(y)) = Self::update_text(cursor_y, &mut self.title, chars, event) {
                    self.cursor_y = Some(y);
                }
            }
            Event::Mouse(_) => {}
            Event::Paste(_) => {}
            Event::Resize(_, _) => {}
        }

        Ok(ControlFlow::Continue(()))
    }

    fn update_text(
        cursor_y: usize,
        text: &mut String,
        chars: usize,
        event: KeyEvent,
    ) -> Option<Option<usize>> {
        if !event.modifiers.difference(KeyModifiers::SHIFT).is_empty() {
            return None;
        }

        Some(match event.code {
            KeyCode::Left => Some(cursor_y.saturating_sub(1)),
            KeyCode::Right if cursor_y < chars => Some(cursor_y + 1),
            KeyCode::Backspace => {
                let y = cursor_y.checked_sub(1)?;
                text.remove(text.char_to_byte_index(y));
                Some(y)
            }
            KeyCode::Delete => {
                let index = text.char_to_byte_index(cursor_y);
                if index < text.len() {
                    text.remove(index);
                }
                return None;
            }
            KeyCode::Enter => None,
            KeyCode::Char(c) => {
                text.insert(text.char_to_byte_index(cursor_y), c);
                Some(cursor_y + 1)
            }
            _ => return None,
        })
    }

    pub fn draw(&self, frame: &mut Frame) {
        let vertical = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Fill(1),
        ]);
        let [title_area, underline_area, main_area] = vertical.areas(frame.area());

        let mut text = Text::raw(self.title.as_str()).bold();
        if self.title.is_empty() {
            text = Text::raw("Neue ToDo Liste").dark_gray().italic();
        }
        frame.render_widget(text, title_area);

        let text = Text::raw("=".repeat(self.title.len())).bold();
        frame.render_widget(text, underline_area);

        let list = List::new(self.todos.iter().map(Todo::to_text));

        frame.render_stateful_widget(list, main_area, &mut self.list_state.borrow_mut());
    }

    pub fn cursor_position(&mut self) -> Option<(u16, u16)> {
        if let Some(y) = self.cursor_y {
            if self.edit_title {
                return Some((u16::try_from(y).unwrap(), 0));
            }
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

#[derive(Debug, Deserialize, Clone, Copy)]
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
    InsertTitle,
    AppendTitle,
    Undo,
    Redo,
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
            (crokey::key! {t}, Self::AppendTitle),
            (crokey::key! {shift-t}, Self::InsertTitle),
            (crokey::key! {u}, Self::Undo),
            (crokey::key! {shift-u}, Self::Redo),
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
                if let Some(state) = model.with_selected_or_select(|t| {
                    let state = t.state;
                    t.state.next();
                    state
                }) {
                    model.push_undo(UndoAction::SetState {
                        index: model.index,
                        state,
                    });
                }
            }
            Self::Indent => {
                if let Some(level) = model.with_selected_or_select(|t| {
                    let level = t.level;
                    t.level_incr();
                    level
                }) {
                    model.push_undo(UndoAction::SetLevel {
                        index: model.index,
                        level,
                    });
                }
            }
            Self::Outdent => {
                if let Some(level) = model.with_selected_or_select(|t| {
                    let level = t.level;
                    t.level_decr();
                    level
                }) {
                    model.push_undo(UndoAction::SetLevel {
                        index: model.index,
                        level,
                    });
                }
            }
            Self::Insert => {
                model.cursor_y = model.with_selected_or_select(|_| 0);
                model.push_undo(UndoAction::SetText {
                    index: model.index,
                    text: model.todos[model.index].text.clone(),
                });
            }
            Self::Append => {
                model.cursor_y = model.with_selected_or_select(|t| t.text.chars().count());
                model.push_undo(UndoAction::SetText {
                    index: model.index,
                    text: model.todos[model.index].text.clone(),
                });
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
                    model.push_undo_delete();
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
                    model.push_undo_delete();
                }
            }
            Self::Delete => {
                model.change_selection(|model| {
                    let todo = model.todos.remove(model.index);
                    model.push_undo(UndoAction::Insert {
                        index: model.index,
                        todo,
                    });
                    if model.index >= model.todos.len() {
                        model.index = model.todos.len().saturating_sub(1);
                    }
                });
            }
            Self::Save => {
                model.save()?;
            }
            Self::InsertTitle => {
                model.edit_title = true;
                model.cursor_y = Some(0);
                model.unselect();
                model.is_selected = false;
            }
            Self::AppendTitle => {
                model.edit_title = true;
                model.cursor_y = Some(model.title.chars().count());
                model.unselect();
                model.is_selected = false;
            }
            Self::Undo => loop {
                if let Some(action) = model.undo_buffer.pop_back() {
                    let redo = action.run(model);
                    model.redo_buffer.push(redo);
                    if model.todos.is_empty() {
                        continue;
                    }
                }
                break;
            },
            Self::Redo => loop {
                if let Some(action) = model.redo_buffer.pop() {
                    let undo = action.run(model);
                    model.undo_buffer.push_back(undo);
                    if model.todos.is_empty() {
                        continue;
                    }
                }
                break;
            },
        }

        Ok(ControlFlow::Continue(()))
    }
}

#[derive(Debug)]
enum UndoAction {
    // undo of insert
    Delete { index: usize },

    // undo of delete
    Insert { index: usize, todo: Todo },

    SetText { index: usize, text: String },

    SetLevel { index: usize, level: usize },

    SetState { index: usize, state: State },
}

impl UndoAction {
    fn run(self, model: &mut Model) -> Self {
        model.unselect();
        model.is_selected = true;
        let reverse = match self {
            Self::Delete { index } => {
                let todo = model.todos.remove(index);
                model.index = if index < model.todos.len() {
                    index
                } else {
                    model.todos.len().saturating_sub(1)
                };
                Self::Insert { index, todo }
            }
            Self::Insert { index, todo } => {
                model.index = index;
                model.todos.insert(index, todo);
                Self::Delete { index }
            }
            Self::SetText { index, text } => {
                model.index = index;
                let text = mem::replace(&mut model.todos[index].text, text);
                Self::SetText { index, text }
            }
            Self::SetLevel { index, level } => {
                model.index = index;
                let level = mem::replace(&mut model.todos[index].level, level);
                Self::SetLevel { index, level }
            }
            Self::SetState { index, state } => {
                model.index = index;
                let state = mem::replace(&mut model.todos[index].state, state);
                Self::SetState { index, state }
            }
        };
        model.reselect();
        reverse
    }
}
