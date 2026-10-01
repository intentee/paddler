use std::sync::Arc;

use llama_cpp_bindings::model::LlamaModel;

use paddler_inference_parameters::inference_parameters::InferenceParameters;

pub struct ContinuousBatchSchedulerContext {
    pub agent_name: Option<String>,
    pub desired_slots_total: u16,
    pub inference_parameters: InferenceParameters,
    pub model: Arc<LlamaModel>,
}
