use std::fmt::Debug;
use std::sync::Arc;

use futures_util::Stream;
use tokio_util::sync::CancellationToken;

use paddler_messaging::inference_client::response::Response as OutgoingResponse;
use paddler_messaging::streamable_result::StreamableResult;

use crate::agent_streaming_request::AgentStreamingRequest;
use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use crate::buffered_request_manager::BufferedRequestManager;
use crate::compatibility::agent_result_stream_event::AgentResultStreamEvent;
use crate::compatibility::agent_result_stream_transformer::AgentResultStreamTransformer;
use crate::inference_service::configuration::Configuration;
use crate::unbounded_stream_from_agent::unbounded_stream_from_agent;
use crate::unbounded_stream_from_agent_params::UnboundedStreamFromAgentParams;

pub struct CompatibilityAppData {
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    pub buffered_request_manager: Arc<BufferedRequestManager>,
    pub inference_service_configuration: Configuration,
    pub shutdown: CancellationToken,
}

impl CompatibilityAppData {
    pub fn agent_result_stream<TParams>(
        &self,
        request_params: TParams,
    ) -> impl Stream<Item = AgentResultStreamEvent<TParams::Response>> + use<TParams>
    where
        TParams: AgentStreamingRequest + Debug + Send + 'static,
        TParams::Response: Debug
            + Into<OutgoingResponse>
            + StreamableResult
            + TryFrom<OutgoingResponse, Error = OutgoingResponse>
            + Send
            + Sync
            + 'static,
    {
        unbounded_stream_from_agent(UnboundedStreamFromAgentParams {
            buffered_request_manager: self.buffered_request_manager.clone(),
            inference_service_configuration: self.inference_service_configuration.clone(),
            request_params,
            shutdown: self.shutdown.clone(),
            transformer: AgentResultStreamTransformer::default(),
        })
    }
}
