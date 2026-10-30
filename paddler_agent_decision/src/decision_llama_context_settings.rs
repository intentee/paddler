use llama_cpp_bindings::context::params::LlamaPoolingType;

use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_runtime::llama_context_settings::LlamaContextSettings;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_agent_runtime::resident_llama_context::ResidentLlamaContext;
use paddler_messaging::inference_capability::InferenceCapability;

use crate::decision_error::DecisionError;

pub struct DecisionLlamaContextSettings<'parameters> {
    pub llama_context_settings: LlamaContextSettings<'parameters>,
}

impl DecisionLlamaContextSettings<'_> {
    pub fn create_llama_context<'model>(
        self,
        loaded_llama_model: &'model LoadedLlamaModel,
        inference_runtime_context: &InferenceRuntimeContext,
    ) -> Result<ResidentLlamaContext<'model>, DecisionError> {
        Ok(loaded_llama_model.create_llama_context(
            inference_runtime_context,
            InferenceCapability::Decisions,
            self.llama_context_settings
                .into_llama_context_params()
                .with_kv_unified(true)
                .with_pooling_type(LlamaPoolingType::None),
        )?)
    }
}
