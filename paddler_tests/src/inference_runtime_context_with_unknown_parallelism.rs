use std::sync::Arc;

use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;

use crate::agent_inference_runtime_context::agent_inference_runtime_context;
use crate::parallelism_that_is_unknown::ParallelismThatIsUnknown;

#[must_use]
pub fn inference_runtime_context_with_unknown_parallelism(slots: u16) -> InferenceRuntimeContext {
    InferenceRuntimeContext {
        available_parallelism: Arc::new(ParallelismThatIsUnknown),
        ..agent_inference_runtime_context(slots)
    }
}
