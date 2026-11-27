use paddler_agent_model_capabilities::agent_pooling_type::AgentPoolingType;
use paddler_agent_model_capabilities::converts_to_llama_pooling_type::ConvertsToLlamaPoolingType as _;
use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_runtime::llama_context_settings::LlamaContextSettings;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_agent_runtime::resident_llama_context::ResidentLlamaContext;
use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_messaging::inference_capability::InferenceCapability;

use crate::embedding_error::EmbeddingError;

pub struct EmbeddingLlamaContextSettings<'parameters> {
    pub llama_context_settings: LlamaContextSettings<'parameters>,
    pub pooling_type: PoolingType,
}

impl EmbeddingLlamaContextSettings<'_> {
    /// # Errors
    ///
    /// Returns [`EmbeddingError::AgentRuntime`] when an embeddings context already occupies the VRAM or llama.cpp cannot create the context.
    pub fn create_llama_context<'model>(
        self,
        loaded_llama_model: &'model LoadedLlamaModel,
        inference_runtime_context: &InferenceRuntimeContext,
    ) -> Result<ResidentLlamaContext<'model>, EmbeddingError> {
        let Self {
            llama_context_settings,
            pooling_type,
        } = self;
        let llama_context_params = llama_context_settings.into_llama_context_params();

        loaded_llama_model
            .create_llama_context(
                inference_runtime_context,
                InferenceCapability::Embeddings,
                llama_context_params
                    .with_embeddings(true)
                    .with_n_ubatch(llama_context_params.n_batch())
                    .with_pooling_type(AgentPoolingType(pooling_type).to_llama_pooling_type()),
            )
            .map_err(EmbeddingError::AgentRuntime)
    }
}
