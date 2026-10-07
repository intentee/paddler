use std::sync::Arc;

use tokio::sync::mpsc;

use paddler_agent_runtime::model_metadata_holder::ModelMetadataHolder;
use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_request_registry::request_registry::RequestRegistry;

use crate::agent_applicable_state_holder::AgentApplicableStateHolder;
use crate::pipeline_request::PipelineRequest;

#[derive(Clone)]
pub struct BalancerMessageContext {
    pub agent_applicable_state_holder: Arc<AgentApplicableStateHolder>,
    pub agent_desired_state_tx: mpsc::UnboundedSender<AgentDesiredState>,
    pub pipeline_request_tx: mpsc::UnboundedSender<PipelineRequest>,
    pub model_metadata_holder: Arc<ModelMetadataHolder>,
    pub request_stoppers: Arc<RequestRegistry<mpsc::UnboundedSender<()>>>,
    pub slot_aggregated_status: Arc<SlotAggregatedStatus>,
}
