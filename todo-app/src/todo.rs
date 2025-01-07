use ratatui::{
    style::Stylize,
    text::{Line, Span, Text},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Todo {
    #[serde(default, skip_serializing_if = "is_zero")]
    pub level: usize,
    pub text: String,
    #[serde(default, skip_serializing_if = "State::is_open")]
    pub state: State,

    #[serde(skip)]
    pub selected: bool,
}

impl Todo {
    const LEVEL_SPACE: &str = "                ";

    pub fn to_text(&self) -> Text {
        let level = Span::raw(&Self::LEVEL_SPACE[..self.level * 2]);
        let state = Span::raw(self.state.as_str());
        let mut text = Span::raw(self.text.as_str());
        if self.text.is_empty() {
            text = Span::raw("Neuer ToDo Punkt").dark_gray().italic();
        }
        if self.selected {
            text = text.underlined();
        }
        let marker = Span::raw(if self.selected { " <==" } else { "" });
        Line::from_iter([level, state, text, marker]).into()
    }

    pub fn level_incr(&mut self) {
        if self.level < const { Self::LEVEL_SPACE.len() / 2 } {
            self.level += 1;
        }
    }

    pub fn level_decr(&mut self) {
        self.level = self.level.saturating_sub(1)
    }
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    #[default]
    Open,
    Wip,
    Done,
}

impl State {
    pub fn next(&mut self) {
        *self = match self {
            Self::Open => Self::Wip,
            Self::Wip => Self::Done,
            Self::Done => Self::Open,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "[ ] ",
            Self::Wip => "[.] ",
            Self::Done => "[X] ",
        }
    }

    fn is_open(&self) -> bool {
        matches!(self, Self::Open)
    }
}

fn is_zero(n: &usize) -> bool {
    *n == 0
}
