use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_runtime::inference_runtime_context_params::InferenceRuntimeContextParams;

#[must_use]
pub fn agent_inference_runtime_context(slots: u16) -> InferenceRuntimeContext {
    InferenceRuntimeContext::new(InferenceRuntimeContextParams {
        agent_name: Some("agent".to_owned()),
        slots,
    })
}
