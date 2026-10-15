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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use actix_web::body::to_bytes;
    use actix_web::http::StatusCode;
    use actix_web::web::Bytes;

    use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

    use super::respond_with_agent_response;
    use crate::agent_controller_fixture::AgentControllerFixture;
    use crate::agent_controller_pool::AgentControllerPool;

    #[tokio::test]
    async fn asking_an_agent_whose_socket_closed_answers_with_an_internal_error() {
        let agent_controller_pool = Arc::new(AgentControllerPool::default());
        let _registration = agent_controller_pool.register_agent_controller(
            AgentControllerFixture::whose_socket_is_gone(
                "agent",
                SlotAggregatedStatusSnapshot::default(),
            ),
        );

        let response =
            respond_with_agent_response(&agent_controller_pool, "agent", |agent_controller| {
                agent_controller.get_model_metadata()
            })
            .await;

        assert_eq!(
            (
                response.status(),
                to_bytes(response.into_body()).await.unwrap()
            ),
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Bytes::from_static(b"The socket of agent agent is closed")
            )
        );
    }
}
