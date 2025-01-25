use anyhow::{Context, Result};
use twitch_api::{
    client::AuthenticatedClient,
    events::{
        chat::{
            message::{ChatMessage, ChatMessageCondition},
            notification::{ChatNotification, ChatNotificationCondition},
        },
        follow::{Follow, FollowCondition},
        stream::{StreamOffline, StreamOfflineCondition, StreamOnline, StreamOnlineCondition},
        subscription::{
            CreateSubscriptionRequest, CreateSubscriptionResponse, DeleteSubscriptionRequest,
            TransportRequest,
        },
        ws::WebSocket,
    },
    secret::Secret,
    user::User,
};

pub struct Subscriptions {
    ids: Vec<Secret>,
}

impl Subscriptions {
    pub async fn subscribe(
        client: &mut AuthenticatedClient,
        user: &User,
    ) -> Result<(Self, WebSocket)> {
        let ws = WebSocket::connect().await?;
        eprintln!("websocket: {:?}", ws.session_id());

        let mut ids = Vec::new();
        let mut push = |res: CreateSubscriptionResponse| -> Result<()> {
            ids.push(
                res.into_subscription()
                    .context("missing subscription info")?
                    .id,
            );
            Ok(())
        };

        let res = client
            .send(&CreateSubscriptionRequest::new::<ChatMessage>(
                &ChatMessageCondition {
                    broadcaster_user_id: user.id.clone(),
                    user_id: user.id.clone(),
                },
                TransportRequest::WebSocket {
                    session_id: ws.session_id().clone(),
                },
            )?)
            .await
            .context("create subscription")?;
        // eprintln!("{res:#?}");
        push(res)?;

        let res = client
            .send(&CreateSubscriptionRequest::new::<ChatNotification>(
                &ChatNotificationCondition {
                    broadcaster_user_id: user.id.clone(),
                    user_id: user.id.clone(),
                },
                TransportRequest::WebSocket {
                    session_id: ws.session_id().clone(),
                },
            )?)
            .await
            .context("create subscription")?;
        // eprintln!("{res:#?}");
        push(res)?;

        let res = client
            .send(&CreateSubscriptionRequest::new::<Follow>(
                &FollowCondition {
                    broadcaster_user_id: user.id.clone(),
                    moderator_user_id: user.id.clone(),
                },
                TransportRequest::WebSocket {
                    session_id: ws.session_id().clone(),
                },
            )?)
            .await
            .context("create subscription")?;
        // eprintln!("{res:#?}");
        push(res)?;

        let res = client
            .send(&CreateSubscriptionRequest::new::<StreamOnline>(
                &StreamOnlineCondition {
                    broadcaster_user_id: user.id.clone(),
                },
                TransportRequest::WebSocket {
                    session_id: ws.session_id().clone(),
                },
            )?)
            .await
            .context("create subscription")?;
        // eprintln!("{res:#?}");
        push(res)?;

        let res = client
            .send(&CreateSubscriptionRequest::new::<StreamOffline>(
                &StreamOfflineCondition {
                    broadcaster_user_id: user.id.clone(),
                },
                TransportRequest::WebSocket {
                    session_id: ws.session_id().clone(),
                },
            )?)
            .await
            .context("create subscription")?;
        // eprintln!("{res:#?}");
        push(res)?;

        eprintln!("subscribed {} ids", ids.len());

        Ok((Self { ids }, ws))
    }

    pub async fn unsubscribe(self, client: &mut AuthenticatedClient) -> Result<()> {
        let n = self.ids.len();
        for id in self.ids {
            client
                .send(&DeleteSubscriptionRequest { id })
                .await
                .context("delete subscription")?;
        }
        eprintln!("unsubscribed {n} ids");
        Ok(())
    }
}
