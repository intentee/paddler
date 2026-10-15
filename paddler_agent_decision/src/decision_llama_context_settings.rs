use llama_cpp_bindings::context::LlamaContext;

use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_runtime::llama_context_settings::LlamaContextSettings;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;

use crate::decision_error::DecisionError;

pub struct DecisionLlamaContextSettings<'parameters> {
    pub llama_context_settings: LlamaContextSettings<'parameters>,
}

impl DecisionLlamaContextSettings<'_> {
    pub fn create_llama_context<'model>(
        self,
        loaded_llama_model: &'model LoadedLlamaModel,
        inference_runtime_context: &InferenceRuntimeContext,
    ) -> Result<LlamaContext<'model>, DecisionError> {
        let mut llama_context = loaded_llama_model.create_llama_context(
            inference_runtime_context,
            self.llama_context_settings
                .into_llama_context_params()
                .with_kv_unified(true),
        )?;

        llama_context
            .enable_masked_nextn_embeddings()
            .map_err(DecisionError::HiddenStatesUnsupported)?;

        Ok(llama_context)
    }
}
