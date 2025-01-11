use std::path::PathBuf;

use clap::{Args, Subcommand};

#[derive(Debug, Args)]
/// Start the main chat
pub struct Run {
    /// Config file path
    #[clap(long, default_value = "twitch-chat.toml")]
    pub config: PathBuf,
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
