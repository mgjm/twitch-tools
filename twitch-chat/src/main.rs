use anyhow::{Context, Result};
use clap::Parser;
use tokio::task::LocalSet;
use twitch_api::{
    auth::{self, Scope},
    client::Client,
    follower::ChannelFollowersRequest,
    user::UsersRequest,
};

mod cmd;

#[derive(Debug, Parser)]
#[clap(version)]
/// Twitch chat in the terminal
enum Cmd {
    Auth(auth::Auth),
    Run(cmd::Run),
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
        Cmd::Auth(cmd) => {
            cmd.run([
                Scope::UserReadChat,
                Scope::UserWriteChat,
                Scope::ModeratorManageAnnouncements,
                Scope::ModeratorReadFollowers,
            ])
            .await
        }
        Cmd::Run(cmd) => cmd.run().await,
    }
}

impl cmd::Run {
    async fn run(&self) -> Result<()> {
        let mut client = Client::new().authenticated_from_env()?;

        let user = client
            .send(&UsersRequest::me())
            .await
            .context("fetch user me")?
            .into_user()
            .context("missing me user")?;

        eprintln!("user id: {:?}", user.id);

        let followers = client
            .send(&ChannelFollowersRequest::total_only(user.id))
            .await
            .context("fetch total followers")?
            .total;

        eprintln!("followers: {followers}");

        Ok(())
    }
}
