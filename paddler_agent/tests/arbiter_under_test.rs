use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use paddler_agent::agent_applicable_state_holder::AgentApplicableStateHolder;
use paddler_agent::pipeline_arbiter::PipelineArbiter;
use paddler_agent::pipeline_arbiter_params::PipelineArbiterParams;
use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_runtime::inference_runtime_context_params::InferenceRuntimeContextParams;
use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;

pub struct ArbiterUnderTest {
    pub inference_runtime_context: InferenceRuntimeContext,
    pub pipeline_arbiter: PipelineArbiter,
    pub served_applicable_state_holder: Arc<AgentApplicableStateHolder>,
}

impl ArbiterUnderTest {
    pub fn new(agent_shutdown: CancellationToken) -> Self {
        let inference_runtime_context =
            InferenceRuntimeContext::new(InferenceRuntimeContextParams {
                agent_name: Some("agent".to_owned()),
                slots: 1,
            });
        let served_applicable_state_holder = Arc::new(AgentApplicableStateHolder::default());

        Self {
            pipeline_arbiter: PipelineArbiter::new(PipelineArbiterParams {
                agent_shutdown,
                inference_runtime_context: inference_runtime_context.clone(),
                served_applicable_state_holder: served_applicable_state_holder.clone(),
            }),
            inference_runtime_context,
            served_applicable_state_holder,
        }
    }

    pub async fn conclude_next_progress(&mut self) -> AgentStateApplicationStatus {
        let serving_startup_progress = self.pipeline_arbiter.next_finished_startup().await;

        self.pipeline_arbiter.conclude(serving_startup_progress);

        self.inference_runtime_context
            .slot_aggregated_status
            .get_state_application_status()
    }
}
