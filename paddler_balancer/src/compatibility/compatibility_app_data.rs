use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::agent_controller_pool::AgentControllerPool;
use crate::agent_relay::AgentRelay;
use crate::agent_streaming_request::AgentStreamingRequest;
use crate::awaiting_agent_relay::AwaitingAgentRelay;
use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use crate::buffered_request_manager::BufferedRequestManager;
use crate::dispatchable_request::DispatchableRequest;
use crate::inference_service::configuration::Configuration;

pub struct CompatibilityAppData {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    pub buffered_request_manager: Arc<BufferedRequestManager>,
    pub inference_service_configuration: Configuration,
    pub shutdown: CancellationToken,
}

impl CompatibilityAppData {
    pub fn agent_relay<TParams>(&self, request_params: TParams) -> AgentRelay<TParams>
    where
        TParams: AgentStreamingRequest,
    {
        AgentRelay::awaiting(AwaitingAgentRelay {
            buffered_request_manager: self.buffered_request_manager.clone(),
            dispatchable_request: DispatchableRequest::from(request_params),
            inference_item_timeout: self.inference_service_configuration.inference_item_timeout,
            shutdown: self.shutdown.clone(),
        })
    }
}
