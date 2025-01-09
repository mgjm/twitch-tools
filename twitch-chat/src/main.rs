use anyhow::{Context, Result};
use clap::Parser;
use tokio::task::LocalSet;
use twitch_api::{
    auth::{self, Scope},
    client::Client,
    events::{
        subscription::{
            CreateSubscriptionRequest, DeleteSubscriptionRequest, GetSubscriptionsRequest,
            TransportRequest,
        },
        ws::WebSocket,
    },
    follower::ChannelFollowersRequest,
    secret::Secret,
    user::UsersRequest,
};

mod cmd;

#[derive(Debug, Parser)]
#[clap(version)]
/// Twitch chat in the terminal
enum Cmd {
    Auth(auth::Auth),
    Run(cmd::Run),
    #[clap(subcommand)]
    Eventsub(cmd::Eventsub),
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
        Cmd::Eventsub(cmd) => cmd.run().await,
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
            .send(&ChannelFollowersRequest::total_only(user.id.clone()))
            .await
            .context("fetch total followers")?
            .total;
        eprintln!("followers: {followers}");

        let mut ws = WebSocket::connect().await?;
        eprintln!("websocket: {:?}", ws.session_id());

        let res = client
            .send(&CreateSubscriptionRequest {
                type_: "channel.chat.message",
                version: "1",
                condition: twitch_api::json!({
                    "broadcaster_user_id": user.id.clone(),
                    "user_id": user.id.clone(),
                }),
                transport: TransportRequest::WebSocket {
                    session_id: ws.session_id().clone(),
                },
            })
            .await
            .context("create subscription")?;
        eprintln!("{res:#?}");

        while ws.next().await?.is_some() {}

        Ok(())
    }
}

impl cmd::Eventsub {
    async fn run(self) -> Result<()> {
        let mut client = Client::new().authenticated_from_env()?;

        match self {
            Self::List {} => {
                let res = client
                    .send(&GetSubscriptionsRequest {
                        ..Default::default()
                    })
                    .await
                    .context("get subscriptions")?;
                eprintln!("{res:#?}");
            }
            Self::Delete { all, id } => {
                let ids = if all {
                    let res = client
                        .send(&GetSubscriptionsRequest {
                            ..Default::default()
                        })
                        .await
                        .context("get subscriptions")?;

                    res.data.into_iter().map(|i| i.id).collect()
                } else {
                    Vec::from_iter(id.map(Secret::new))
                };

                let num_ids = ids.len();
                for id in ids {
                    client
                        .send(&DeleteSubscriptionRequest { id })
                        .await
                        .context("delete subscription")?;
                }

                eprintln!("deleted {num_ids} ids",);
            }
        }

        Ok(())
    }
}
