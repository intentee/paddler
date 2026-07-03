use std::sync::Arc;

use actix_web::rt;
use actix_ws::Session;
use log::error;
use paddler_messaging::inference_client::message::Message as OutgoingMessage;
use paddler_messaging::inference_client::notification::Notification;
use paddler_messaging::subscribes_to_updates::SubscribesToUpdates as _;
use tokio_util::sync::CancellationToken;

use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use crate::cluster_prompting_mode::ClusterPromptingMode;
use crate::controls_session::ControlsSession as _;
use crate::websocket_session_controller::WebSocketSessionController;

async fn send_notification(
    session_controller: &mut WebSocketSessionController<OutgoingMessage>,
    notification: Notification,
) -> bool {
    match session_controller
        .send_response(OutgoingMessage::Notification(notification))
        .await
    {
        Ok(()) => true,
        Err(err) => {
            error!("Failed to push prompting mode notification: {err}");

            false
        }
    }
}

pub fn spawn_prompting_mode_watcher(
    balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    connection_close: CancellationToken,
    session: Session,
) {
    rt::spawn(async move {
        let mut update_rx = balancer_applicable_state_holder.subscribe_to_updates();
        let mut session_controller = WebSocketSessionController::<OutgoingMessage>::new(session);
        let mut last_mode =
            ClusterPromptingMode::from_applicable_state_holder(&balancer_applicable_state_holder);

        if last_mode == ClusterPromptingMode::DisabledForEmbeddings
            && !send_notification(&mut session_controller, Notification::PromptingDisabled).await
        {
            return;
        }

        loop {
            tokio::select! {
                () = connection_close.cancelled() => break,
                changed = update_rx.changed() => {
                    if changed.is_err() {
                        break;
                    }

                    let current_mode = ClusterPromptingMode::from_applicable_state_holder(
                        &balancer_applicable_state_holder,
                    );

                    if current_mode == last_mode {
                        continue;
                    }

                    last_mode = current_mode;

                    let notification = match current_mode {
                        ClusterPromptingMode::Enabled => Notification::PromptingEnabled,
                        ClusterPromptingMode::DisabledForEmbeddings => {
                            Notification::PromptingDisabled
                        }
                    };

                    if !send_notification(&mut session_controller, notification).await {
                        break;
                    }
                }
            }
        }
    });
}
