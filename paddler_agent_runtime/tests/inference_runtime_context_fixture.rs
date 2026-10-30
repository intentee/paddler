use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_runtime::inference_runtime_context_params::InferenceRuntimeContextParams;

pub fn inference_runtime_context_fixture() -> InferenceRuntimeContext {
    InferenceRuntimeContext::new(InferenceRuntimeContextParams {
        agent_name: Some("agent".to_owned()),
        slots: 1,
    })
}
