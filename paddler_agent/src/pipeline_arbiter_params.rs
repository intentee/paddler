use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;

use crate::agent_applicable_state_holder::AgentApplicableStateHolder;

pub struct PipelineArbiterParams {
    pub agent_shutdown: CancellationToken,
    pub inference_runtime_context: InferenceRuntimeContext,
    pub served_applicable_state_holder: Arc<AgentApplicableStateHolder>,
}
