use std::{env, fs, io, path::PathBuf, time::Duration};

use crossterm::event::{self, Event};

use crate::model::Model;

mod model;
mod todo;

fn main() {
    let path: PathBuf = env::args_os()
        .nth(1)
        .expect("missing data path argument")
        .into();
    let data = fs::read_to_string(&path).expect("open data file");
    let mut model: Model = toml::from_str(&data).expect("failed to parse data");
    model.path = path;

    let mut terminal = ratatui::init();
    let _tty_mode_guard = TtyModes::enable();
    loop {
        terminal
            .draw(|frame| model.draw(frame))
            .expect("failed to draw frame");

        if let Some(pos) = model.cursor_position() {
            terminal
                .set_cursor_position(pos)
                .expect("set cusrsor position");
            terminal.show_cursor().expect("show cursor");
        }
        if model.update(read_event(model.timeout)).is_break() {
            break;
        }
    }

    ratatui::restore();

    model.save();
}

fn read_event(timeout: Option<Duration>) -> Option<Event> {
    if let Some(timeout) = timeout {
        if !event::poll(timeout).expect("failed to poll event") {
            return None;
        }
    }
    Some(event::read().expect("failed to read event"))
}

#[must_use]
struct TtyModes(());

impl TtyModes {
    fn enable() -> Self {
        crossterm::execute!(io::stdout(), event::EnableFocusChange).expect("enable tty modes");
        Self(())
    }
}

impl Drop for TtyModes {
    fn drop(&mut self) {
        if let Err(err) = crossterm::execute!(io::stdout(), event::DisableFocusChange) {
            eprintln!("failed to disable tty modes: {err}");
        }
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
