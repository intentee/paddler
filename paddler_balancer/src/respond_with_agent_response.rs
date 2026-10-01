use std::sync::Arc;

use actix_web::HttpResponse;
use serde::Serialize;
use tokio::select;
use tokio::time::Duration;
use tokio::time::sleep;

use crate::agent_controller::AgentController;
use crate::agent_controller_pool::AgentControllerPool;
use crate::agent_relay_error::AgentRelayError;
use crate::agent_response_receiver::AgentResponseReceiver;

const AGENT_RESPONSE_TIMEOUT: Duration = Duration::from_secs(3);

pub async fn respond_with_agent_response<TResponse, TAskAgent>(
    agent_controller_pool: &AgentControllerPool,
    agent_id: &str,
    ask_agent: TAskAgent,
) -> HttpResponse
where
    TResponse: Serialize,
    TAskAgent:
        FnOnce(Arc<AgentController>) -> Result<AgentResponseReceiver<TResponse>, AgentRelayError>,
{
    let Some(agent_controller) = agent_controller_pool.get_agent_controller(agent_id) else {
        return HttpResponse::NotFound().finish();
    };

    let connection_close = agent_controller.connection_close.clone();

    match ask_agent(agent_controller) {
        Ok(mut agent_response_receiver) => select! {
            () = connection_close.cancelled() => HttpResponse::BadGateway().finish(),
            () = sleep(AGENT_RESPONSE_TIMEOUT) => HttpResponse::GatewayTimeout().finish(),
            Some(agent_response) = agent_response_receiver.response_rx.recv() => {
                HttpResponse::Ok().json(agent_response)
            }
        },
        Err(err) => HttpResponse::InternalServerError().body(format!("{err}")),
    }
}
