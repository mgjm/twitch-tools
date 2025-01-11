use std::path::PathBuf;

use clap::{Args, Subcommand};

#[derive(Debug, Args)]
/// Start the main chat
pub struct Run {
    #[clap(long)]
    /// Pulse audio output device
    pub device: Option<String>,

    #[clap(long)]
    /// Output volume
    pub volume: Option<f32>,

    /// Path to an audio file
    pub path: PathBuf,
}

#[derive(Debug, Subcommand)]
/// Manage event subscriptions
pub enum Eventsub {
    /// List all subscriptions
    List {},

    /// Delete subsciptions
    Delete {
        /// Delete all subscriptions
        #[clap(long)]
        all: bool,

        /// Subscription ids to delete
        #[clap(required_unless_present = "all")]
        id: Option<String>,
    },
}
