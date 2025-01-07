use anyhow::Result;
use clap::Parser;
use tokio::task::LocalSet;
use twitch_api::auth;

#[derive(Debug, Parser)]
#[clap(version)]
/// Example twitch api client
enum Cmd {
    Version(cmd::Version),
    Auth(auth::Auth),
}

fn main() -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(LocalSet::new().run_until(run()))
}

async fn run() -> Result<()> {
    let cmd = Cmd::parse();

    match cmd {
        Cmd::Version(cmd) => cmd.run(),
        Cmd::Auth(cmd) => cmd.run().await,
    }
}

impl cmd::Version {
    fn run(&self) -> Result<()> {
        todo!()
    }
}

mod cmd {
    use clap::Args;

    #[derive(Debug, Args)]
    /// Show twitch api version
    pub struct Version {}
}
