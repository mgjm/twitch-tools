use std::{env, fs, io, path::PathBuf, time::Duration};

use anyhow::{Context, Result};
use config::Config;
use crossterm::event::{self, Event};
use ratatui::DefaultTerminal;

use crate::model::Model;

mod config;
mod model;
mod todo;

fn main() -> Result<()> {
    let config = Config::load_env()?;

    let path: PathBuf = env::args_os()
        .nth(1)
        .context("missing data path argument")?
        .into();
    let data = fs::read_to_string(&path)
        .or_else(|err| {
            if err.kind() == io::ErrorKind::NotFound {
                Ok(String::new())
            } else {
                Err(err)
            }
        })
        .context("open data file")?;
    let mut model: Model = toml::from_str(&data).context("parse data")?;
    model.path = path;
    model.keybindings.extend(config.keybindings);
    model.max_undo = config.undo_steps;

    model.did_load();

    let terminal = ratatui::init();
    let _tty_mode_guard = TtyModes::enable();
    let run_result = run(&mut model, terminal);

    ratatui::restore();

    model.save()?;

    run_result
}

fn run(model: &mut Model, mut terminal: DefaultTerminal) -> Result<(), anyhow::Error> {
    loop {
        terminal
            .draw(|frame| model.draw(frame))
            .context("draw frame")?;

        if let Some(pos) = model.cursor_position() {
            terminal
                .set_cursor_position(pos)
                .context("set cusrsor position")?;
            terminal.show_cursor().context("show cursor")?;
        }
        if model.update(read_event(model.timeout)?)?.is_break() {
            break Ok(());
        }
    }
}

fn read_event(timeout: Option<Duration>) -> Result<Option<Event>> {
    if let Some(timeout) = timeout {
        if !event::poll(timeout).context("poll event")? {
            return Ok(None);
        }
    }
    Ok(Some(event::read().context("read event")?))
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
