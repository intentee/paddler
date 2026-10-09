use std::sync::Arc;

use tokio::sync::mpsc;

use paddler_agent_runtime::model_metadata_holder::ModelMetadataHolder;
use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_messaging::agent_desired_state::AgentDesiredState;

use crate::agent_applicable_state_holder::AgentApplicableStateHolder;
use crate::balancer_message_context::BalancerMessageContext;
use crate::pipeline_request::PipelineRequest;

const FIXTURE_SLOTS: u16 = 1;

pub struct BalancerMessageContextFixture {
    pub agent_desired_state_rx: mpsc::UnboundedReceiver<AgentDesiredState>,
    pub context: BalancerMessageContext,
    pub pipeline_request_rx: mpsc::UnboundedReceiver<PipelineRequest>,
}

impl Default for BalancerMessageContextFixture {
    fn default() -> Self {
        let (agent_desired_state_tx, agent_desired_state_rx) = mpsc::unbounded_channel();
        let (pipeline_request_tx, pipeline_request_rx) = mpsc::unbounded_channel();

        Self {
            agent_desired_state_rx,
            context: BalancerMessageContext {
                agent_applicable_state_holder: Arc::new(AgentApplicableStateHolder::default()),
                agent_desired_state_tx,
                pipeline_request_tx,
                model_metadata_holder: Arc::new(ModelMetadataHolder::new()),
                request_stoppers: Arc::default(),
                slot_aggregated_status: Arc::new(SlotAggregatedStatus::new(FIXTURE_SLOTS)),
            },
            pipeline_request_rx,
        }
    }
}
