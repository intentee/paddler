use std::sync::Arc;

use actix_web::rt;
use tokio::select;
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;

use paddler_messaging::inference_client::message::Message as OutgoingMessage;
use paddler_messaging::inference_client::notification::Notification;

use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use crate::controls_session::ControlsSession as _;
use crate::websocket_outbound_frame::WebSocketOutboundFrame;
use crate::websocket_session_controller::WebSocketSessionController;

pub async fn report_cluster_inference_mode(
    balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    connection_close: CancellationToken,
    outbound_tx: UnboundedSender<WebSocketOutboundFrame>,
) {
    let mut balancer_applicable_state_rx = balancer_applicable_state_holder.subscribe();
    let mut session_controller = WebSocketSessionController::<OutgoingMessage>::new(outbound_tx);
    let mut last_inference_mode = balancer_applicable_state_rx
        .borrow_and_update()
        .inference_mode();

    session_controller
        .send_response_safe(OutgoingMessage::Notification(
            Notification::ClusterInferenceMode(last_inference_mode),
        ))
        .await;

    rt::spawn(async move {
        loop {
            select! {
                () = connection_close.cancelled() => break,
                Ok(()) = balancer_applicable_state_rx.changed() => {
                    let current_inference_mode = balancer_applicable_state_rx
                        .borrow_and_update()
                        .inference_mode();

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
