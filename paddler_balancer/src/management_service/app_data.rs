use std::sync::Arc;

use actix_web::HttpResponse;
use serde::Serialize;
use tokio::select;
use tokio::time::Duration;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use crate::agent_controller::AgentController;
use crate::agent_controller_pool::AgentControllerPool;
use crate::agent_response_receiver::AgentResponseReceiver;
use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use crate::balancer_desired_state_store::BalancerDesiredStateStore;
use crate::buffered_request_manager::BufferedRequestManager;

const AGENT_RESPONSE_TIMEOUT: Duration = Duration::from_secs(3);

pub struct AppData {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    pub balancer_desired_state_store: Arc<BalancerDesiredStateStore>,
    pub buffered_request_manager: Arc<BufferedRequestManager>,
    pub shutdown: CancellationToken,
    pub statsd_prefix: String,
}

impl AppData {
    pub async fn respond_with_agent_response<TResponse, TAskAgent>(
        &self,
        agent_id: &str,
        ask_agent: TAskAgent,
    ) -> HttpResponse
    where
        TResponse: Serialize,
        TAskAgent: FnOnce(Arc<AgentController>) -> AgentResponseReceiver<TResponse>,
    {
        let Some(agent_controller) = self.agent_controller_pool.get_agent_controller(agent_id)
        else {
            return HttpResponse::NotFound().finish();
        };

        let connection_close = agent_controller.connection_close.clone();

        let mut agent_response_receiver = ask_agent(agent_controller);

        select! {
            () = connection_close.cancelled() => HttpResponse::BadGateway().finish(),
            () = sleep(AGENT_RESPONSE_TIMEOUT) => HttpResponse::GatewayTimeout().finish(),
            Some(agent_response) = agent_response_receiver.response_rx.recv() => {
                HttpResponse::Ok().json(agent_response)
            }
        }
    }
}
