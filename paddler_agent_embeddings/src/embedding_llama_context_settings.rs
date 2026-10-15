use llama_cpp_bindings::context::LlamaContext;

use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_runtime::llama_context_settings::LlamaContextSettings;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_inference_parameters::pooling_type::PoolingType;

use crate::agent_pooling_type::AgentPoolingType;
use crate::converts_to_llama_pooling_type::ConvertsToLlamaPoolingType as _;

pub struct EmbeddingLlamaContextSettings<'parameters> {
    pub llama_context_settings: LlamaContextSettings<'parameters>,
    pub pooling_type: PoolingType,
}

impl EmbeddingLlamaContextSettings<'_> {
    pub fn create_llama_context<'model>(
        self,
        loaded_llama_model: &'model LoadedLlamaModel,
        inference_runtime_context: &InferenceRuntimeContext,
    ) -> Result<LlamaContext<'model>, AgentRuntimeError> {
        let Self {
            llama_context_settings,
            pooling_type,
        } = self;
        let n_ubatch = llama_context_settings
            .model_runtime_parameters
            .n_batch
            .tokens()
            .get();

        loaded_llama_model.create_llama_context(
            inference_runtime_context,
            llama_context_settings
                .into_llama_context_params()
                .with_embeddings(true)
                .with_n_ubatch(n_ubatch)
                .with_pooling_type(AgentPoolingType(pooling_type).to_llama_pooling_type()),
        )
    }
}
