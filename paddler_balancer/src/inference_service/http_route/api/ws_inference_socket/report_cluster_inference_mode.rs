use std::sync::Arc;

use actix_web::rt;
use actix_ws::Session;
use tokio::select;
use tokio_util::sync::CancellationToken;

use paddler_messaging::inference_client::message::Message as OutgoingMessage;
use paddler_messaging::inference_client::notification::Notification;
use paddler_messaging::subscribes_to_updates::SubscribesToUpdates as _;

use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use crate::controls_session::ControlsSession as _;
use crate::websocket_session_controller::WebSocketSessionController;

pub async fn report_cluster_inference_mode(
    balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    connection_close: CancellationToken,
    session: Session,
) {
    let mut update_rx = balancer_applicable_state_holder.subscribe_to_updates();
    let mut session_controller = WebSocketSessionController::<OutgoingMessage>::new(session);
    let mut last_inference_mode = balancer_applicable_state_holder.inference_mode();

    session_controller
        .send_response_safe(OutgoingMessage::Notification(
            Notification::ClusterInferenceMode(last_inference_mode),
        ))
        .await;

    rt::spawn(async move {
        loop {
            select! {
                () = connection_close.cancelled() => break,
                Ok(()) = update_rx.changed() => {
                    let current_inference_mode = balancer_applicable_state_holder.inference_mode();

                    if current_inference_mode == last_inference_mode {
                        continue;
                    }

                    last_inference_mode = current_inference_mode;

                    session_controller
                        .send_response_safe(OutgoingMessage::Notification(
                            Notification::ClusterInferenceMode(current_inference_mode),
                        ))
                        .await;
                }
            }
        }
    });
}
