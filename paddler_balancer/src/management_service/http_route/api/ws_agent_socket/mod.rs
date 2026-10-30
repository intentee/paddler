mod agent_socket_controller_context;
mod agent_socket_registration;

use std::future::Future;
use std::future::ready;
use std::sync::Arc;

use actix_web::Error;
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use actix_web::web::Data;
use actix_web::web::Path;
use actix_web::web::Payload;
use actix_web::web::ServiceConfig;
use actix_web::web::get;
use anyhow::Result;
use parking_lot::RwLock;
use serde_json::Error as SerdeJsonError;
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;

use paddler_messaging::api_path::ApiPath;
use paddler_messaging::management_socket::agent::message::Message as AgentJsonRpcMessage;
use paddler_messaging::management_socket::agent::notification::Notification as AgentJsonRpcNotification;
use paddler_messaging::management_socket::agent::notification_params::version_params::VersionParams;
use paddler_messaging::management_socket::balancer::message::Message as ManagementJsonRpcMessage;

use self::agent_socket_controller_context::AgentSocketControllerContext;
use self::agent_socket_registration::AgentSocketRegistration;
use crate::agent_controller_pool::AgentControllerPool;
use crate::agent_id_path_params::AgentIdPathParams;
use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use crate::continuation_decision::ContinuationDecision;
use crate::controls_session::ControlsSession as _;
use crate::controls_websocket_endpoint::ControlsWebSocketEndpoint;
use crate::management_service::app_data::AppData;
use crate::websocket_close_cause::WebSocketCloseCause;
use crate::websocket_outbound_frame::WebSocketOutboundFrame;
use crate::websocket_session_controller::WebSocketSessionController;

async fn respond(
    app_data: Data<AppData>,
    path_params: Path<AgentIdPathParams>,
    payload: Payload,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let agent_socket_controller = AgentSocketController {
        agent_controller_pool: app_data.agent_controller_pool.clone(),
        agent_id: path_params.agent_id.clone(),
        balancer_applicable_state_holder: app_data.balancer_applicable_state_holder.clone(),
    };

    agent_socket_controller.respond(payload, req, app_data.shutdown.clone())
}

struct AgentSocketController {
    agent_controller_pool: Arc<AgentControllerPool>,
    agent_id: String,
    balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
}

impl ControlsWebSocketEndpoint for AgentSocketController {
    type Context = AgentSocketControllerContext;
    type IncomingMessage = ManagementJsonRpcMessage;
    type OutgoingMessage = AgentJsonRpcMessage;

    fn create_context(&self) -> Self::Context {
        AgentSocketControllerContext {
            agent_controller_pool: self.agent_controller_pool.clone(),
            agent_id: self.agent_id.clone(),
            balancer_applicable_state_holder: self.balancer_applicable_state_holder.clone(),
            registration: RwLock::new(AgentSocketRegistration::AwaitingRegistration),
        }
    }

    fn handle_deserialized_message(
        connection_close: CancellationToken,
        context: Arc<Self::Context>,
        deserialized_message: Self::IncomingMessage,
        websocket_session_controller: WebSocketSessionController<Self::OutgoingMessage>,
    ) -> impl Future<Output = ContinuationDecision> + Send {
        ready(context.handle_message(
            connection_close,
            deserialized_message,
            websocket_session_controller,
        ))
    }

    fn handle_undeserializable_message(
        _text: &str,
        _deserialization_error: SerdeJsonError,
        _websocket_session_controller: WebSocketSessionController<Self::OutgoingMessage>,
    ) -> impl Future<Output = ContinuationDecision> + Send {
        ready(ContinuationDecision::Stop(
            WebSocketCloseCause::AgentMessageUndeserializable,
        ))
    }

    async fn on_connection_start(
        _connection_close: CancellationToken,
        _context: Arc<Self::Context>,
        outbound_tx: &UnboundedSender<WebSocketOutboundFrame>,
    ) {
        WebSocketSessionController::new(outbound_tx.clone())
            .send_response_safe(AgentJsonRpcMessage::Notification(
                AgentJsonRpcNotification::Version(VersionParams {
                    version: env!("CARGO_PKG_VERSION").to_owned(),
                }),
            ))
            .await;
    }
}

pub fn ws_agent_socket(cfg: &mut ServiceConfig) {
    cfg.route(
        &ApiPath::agent_socket(AgentIdPathParams::ROUTE_SEGMENT),
        get().to(respond),
    );
}
