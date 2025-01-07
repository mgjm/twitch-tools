use std::{cell::RefCell, fs, ops::ControlFlow, path::PathBuf, time::Duration};

use crossterm::event::{Event, KeyCode};
use ratatui::{
    layout::{Constraint, Layout},
    style::Stylize,
    text::Text,
    widgets::{List, ListState},
    Frame,
};
use serde::{Deserialize, Serialize};

use crate::{todo::Todo, CharToByteIndex};

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Model {
    title: String,

    #[serde(default, rename = "todo")]
    pub todos: Vec<Todo>,

    #[serde(skip)]
    pub path: PathBuf,

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

    pub fn save(&self) {
        fs::write(
            self.path.as_path(),
            toml::to_string(self).expect("failed to serialize data"),
        )
        .expect("failed to write data");
    }

    pub fn update(&mut self, event: Option<Event>) -> ControlFlow<()> {
        if let Some(cursor_y) = self.cursor_y {
            self.update_insert(event, cursor_y);
            return ControlFlow::Continue(());
        }

        if matches!(event, Some(Event::Key(key)) if key.code == KeyCode::Char('q')) {
            return ControlFlow::Break(());
        }

        if let Some(event) = event {
            self.update_inner(event);
        } else {
            self.update_timeout();
        }

        ControlFlow::Continue(())
    }

    fn update_inner(&mut self, event: Event) {
        let list_state = self.list_state.get_mut();
        match event {
            Event::FocusGained => {}
            Event::FocusLost => {
                with_selected(list_state, &mut self.todos, |t| t.selected = false);
                list_state.select(None);
            }
            Event::Key(event) => match event.code {
                KeyCode::Char('j') => {
                    with_selected(list_state, &mut self.todos, |t| t.selected = false);
                    if list_state.selected().is_none() {
                        list_state.select(self.prev_selected);
                    }
                    list_state.select_next();
                    with_selected(list_state, &mut self.todos, |t| t.selected = true);
                }
                KeyCode::Char('k') => {
                    with_selected(list_state, &mut self.todos, |t| t.selected = false);
                    if list_state.selected().is_none() {
                        list_state.select(self.prev_selected);
                    }
                    list_state.select_previous();
                    with_selected(list_state, &mut self.todos, |t| t.selected = true);
                }
                KeyCode::Esc => {
                    with_selected(list_state, &mut self.todos, |t| t.selected = false);
                    if list_state.selected().is_none() {
                        list_state.select(self.prev_selected);
                    } else {
                        list_state.select(None);
                    }
                    with_selected(list_state, &mut self.todos, |t| t.selected = true);
                }
                KeyCode::Char(' ') => {
                    with_selected(list_state, &mut self.todos, |t| t.state.next());
                    if list_state.selected().is_none() {
                        list_state.select(self.prev_selected);
                        with_selected(list_state, &mut self.todos, |t| t.selected = true);
                    }
                }
                KeyCode::Char('>') => {
                    with_selected(list_state, &mut self.todos, |t| t.level_incr());
                    if list_state.selected().is_none() {
                        list_state.select(self.prev_selected);
                        with_selected(list_state, &mut self.todos, |t| t.selected = true);
                    }
                }
                KeyCode::Char('<') => {
                    with_selected(list_state, &mut self.todos, |t| {
                        t.level_decr();
                    });
                    if list_state.selected().is_none() {
                        list_state.select(self.prev_selected);
                        with_selected(list_state, &mut self.todos, |t| t.selected = true);
                    }
                }
                KeyCode::Char('i') => {
                    self.cursor_y = Some(0);
                }
                KeyCode::Char('a') => {
                    if let Some(index) = list_state.selected() {
                        if let Some(todo) = self.todos.get(index) {
                            self.cursor_y = Some(todo.text.chars().count());
                        }
                    }
                }
                KeyCode::Char('o') => {
                    with_selected(list_state, &mut self.todos, |t| t.selected = false);
                    if let Some(index) = list_state.selected() {
                        if let Some(todo) = self.todos.get(index) {
                            self.todos.insert(
                                index + 1,
                                Todo {
                                    level: todo.level,
                                    ..Default::default()
                                },
                            );
                            list_state.select(Some(index + 1));
                            self.cursor_y = Some(0);
                        }
                    }
                    with_selected(list_state, &mut self.todos, |t| t.selected = true);
                }
                KeyCode::Char('O') => {
                    with_selected(list_state, &mut self.todos, |t| t.selected = false);
                    if let Some(index) = list_state.selected() {
                        if let Some(todo) = self.todos.get(index) {
                            self.todos.insert(
                                index,
                                Todo {
                                    level: todo.level,
                                    ..Default::default()
                                },
                            );
                            self.cursor_y = Some(0);
                        }
                    }
                    with_selected(list_state, &mut self.todos, |t| t.selected = true);
                }
                KeyCode::Char('d') => {
                    with_selected(list_state, &mut self.todos, |t| t.selected = false);
                    if let Some(index) = list_state.selected() {
                        if index < self.todos.len() {
                            self.todos.remove(index);
                        }
                    }
                    with_selected(list_state, &mut self.todos, |t| t.selected = true);
                }
                KeyCode::Char('s') => {
                    self.save();
                }
                _ => {}
            },
            Event::Mouse(_) => {}
            Event::Paste(_) => {}
            Event::Resize(_, _) => {}
        }
        if let Some(index) = self.list_state.get_mut().selected() {
            self.prev_selected = Some(index);
            self.timeout = Some(Duration::from_secs(10));
        }
    }

    fn update_timeout(&mut self) {
        let list_state = self.list_state.get_mut();

        with_selected(list_state, &mut self.todos, |t| t.selected = false);
        list_state.select(None);
        self.timeout = None;
    }

    fn update_insert(&mut self, event: Option<Event>, cursor_y: usize) {
        self.timeout = None;
        let Some(event) = event else {
            return;
        };

        let list_state = self.list_state.get_mut();
        let Some(index) = list_state.selected() else {
            self.cursor_y = None;
            return;
        };
        let Some(todo) = self.todos.get_mut(index) else {
            self.cursor_y = None;
            return;
        };

        match event {
            Event::FocusGained => {}
            Event::FocusLost => {}
            Event::Key(event) => match event.code {
                KeyCode::Esc => {
                    self.cursor_y = None;
                }
                KeyCode::Left => {
                    self.cursor_y = Some(cursor_y.saturating_sub(1));
                }
                KeyCode::Right if cursor_y < todo.text.chars().count() => {
                    self.cursor_y = Some(cursor_y + 1);
                }
                KeyCode::Backspace => {
                    let Some(y) = cursor_y.checked_sub(1) else {
                        return;
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
            },
            Event::Mouse(_) => {}
            Event::Paste(_) => {}
            Event::Resize(_, _) => {}
        }
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
