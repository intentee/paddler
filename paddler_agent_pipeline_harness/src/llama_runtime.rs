use std::path::Path;
use std::sync::Arc;

use anyhow::Context as _;
use anyhow::Result as AnyhowResult;
use llama_cpp_context::llama_batch::LlamaBatch;
use llama_cpp_context::llama_context::LlamaContext;
use llama_cpp_decision::decision_model::DecisionModel;
use llama_cpp_model::add_bos::AddBos;
use llama_cpp_model::parse_special_tokens::ParseSpecialTokens;

use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_decision::decision_llama_context_settings::DecisionLlamaContextSettings;
use paddler_agent_embeddings::embedding_error::EmbeddingError;
use paddler_agent_embeddings::embedding_llama_context_settings::EmbeddingLlamaContextSettings;
use paddler_agent_model_capabilities::established_model_capabilities::EstablishedModelCapabilities;
use paddler_agent_model_capabilities::model_capabilities_settings::ModelCapabilitiesSettings;
use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_runtime::llama_context_settings::LlamaContextSettings;
use paddler_agent_runtime::llama_model_settings::LlamaModelSettings;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_agent_runtime::model_placement::ModelPlacement;
use paddler_agent_runtime::resident_llama_context::ResidentLlamaContext;
use paddler_agent_text_generation::constants::LAST_LOGITS_BATCH_POSITION;
use paddler_agent_text_generation::sampler_chain_factory::SamplerChainFactory;
use paddler_inference_parameters::desired_model_runtime_parameters::DesiredModelRuntimeParameters;
use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_messaging::inference_capability::InferenceCapability;
use paddler_model_card_fixture::served_model_runtime_parameters::served_model_runtime_parameters;

use crate::prompt_sampling_positions::PromptSamplingPositions;

pub struct LlamaRuntime {
    pub inference_runtime_context: InferenceRuntimeContext,
    pub loaded_llama_model: LoadedLlamaModel,
    pub model_runtime_parameters: DesiredModelRuntimeParameters,
}

impl LlamaRuntime {
    /// # Errors
    ///
    /// Returns the [`AgentRuntimeError`] of [`LoadedLlamaModel::load`] when the model at `model_path` cannot be loaded.
    pub fn load(
        model_path: &Path,
        inference_runtime_context: InferenceRuntimeContext,
    ) -> Result<Self, AgentRuntimeError> {
        let model_runtime_parameters = served_model_runtime_parameters();
        let loaded_llama_model = LoadedLlamaModel::load(
            &inference_runtime_context,
            LlamaModelSettings {
                model_path: model_path.to_path_buf(),
                placement: ModelPlacement::from(&model_runtime_parameters),
            },
        )?;

        Ok(Self {
            inference_runtime_context,
            loaded_llama_model,
            model_runtime_parameters,
        })
    }

    /// # Errors
    ///
    /// Returns [`DecisionError::AgentRuntime`] when llama.cpp cannot create the decision context or a decision context already occupies the VRAM.
    pub fn create_decision_llama_context(
        &self,
        decision_model: &DecisionModel,
        model_capabilities: &EstablishedModelCapabilities,
        n_seq_max: u32,
    ) -> Result<ResidentLlamaContext<'_>, DecisionError> {
        DecisionLlamaContextSettings {
            decision_model,
            llama_context_settings: self.llama_context_settings(n_seq_max),
            model_capabilities,
        }
        .create_llama_context(&self.loaded_llama_model, &self.inference_runtime_context)
    }

    /// # Errors
    ///
    /// Returns [`EmbeddingError::AgentRuntime`] when llama.cpp cannot create the embedding context or an embedding context already occupies the VRAM.
    pub fn create_embedding_llama_context(
        &self,
        n_seq_max: u32,
        pooling_type: PoolingType,
    ) -> Result<ResidentLlamaContext<'_>, EmbeddingError> {
        EmbeddingLlamaContextSettings {
            llama_context_settings: self.llama_context_settings(n_seq_max),
            pooling_type,
        }
        .create_llama_context(&self.loaded_llama_model, &self.inference_runtime_context)
    }

    /// # Errors
    ///
    /// Returns [`AgentRuntimeError::LlamaContextAlreadyResident`] when a text generation context already occupies the VRAM, and [`AgentRuntimeError::ContextCreationFailed`] when llama.cpp cannot create the context.
    pub fn create_text_generation_llama_context(
        &self,
        n_seq_max: u32,
    ) -> Result<ResidentLlamaContext<'_>, AgentRuntimeError> {
        self.loaded_llama_model.create_llama_context(
            &self.inference_runtime_context,
            InferenceCapability::TextGeneration,
            self.llama_context_settings(n_seq_max)
                .into_llama_context_params(),
        )
    }

    /// # Errors
    ///
    /// Returns an error when the prompt does not tokenize, does not fit the batch, or does not decode.
    pub fn decode_prompt_for_sampling(
        &self,
        llama_context: &mut LlamaContext<'_>,
        prompt: &str,
    ) -> AnyhowResult<PromptSamplingPositions> {
        self.loaded_llama_model
            .model
            .str_to_token(prompt, AddBos::Always, ParseSpecialTokens::Always)
            .context("the prompt must tokenize")
            .and_then(|prompt_tokens| {
                let mut batch = LlamaBatch::new(llama_context.limits());

                batch
                    .add_sequence(&prompt_tokens, 0, false)
                    .context("the prompt must fit the batch")
                    .and_then(|()| {
                        llama_context
                            .decode(&batch)
                            .context("the prompt must decode")
                    })
                    .map(|()| PromptSamplingPositions {
                        batch_position: LAST_LOGITS_BATCH_POSITION,
                        next_token_position: batch.n_tokens(),
                    })
            })
    }

    /// # Errors
    ///
    /// Returns an error when the chat template, the multimodal projection or the capabilities of the loaded model cannot be established.
    pub fn establish_model_capabilities(
        &self,
        model_capabilities_settings: ModelCapabilitiesSettings,
    ) -> AnyhowResult<Arc<EstablishedModelCapabilities>> {
        EstablishedModelCapabilities::establish(
            &self.loaded_llama_model,
            &self.inference_runtime_context,
            model_capabilities_settings,
        )
        .map(Arc::new)
        .context("the capabilities of the loaded model must be established")
    }

    #[must_use]
    pub fn n_batch_tokens(&self) -> usize {
        self.llama_context_settings(1)
            .into_llama_context_params()
            .n_batch() as usize
    }

    #[must_use]
    pub fn sampler_chain_factory(
        &self,
        EstablishedModelCapabilities {
            sampling_defaults, ..
        }: &EstablishedModelCapabilities,
    ) -> SamplerChainFactory {
        SamplerChainFactory {
            sampling_parameters: *sampling_defaults,
            n_vocab: self.loaded_llama_model.model.n_vocab(),
        }
    }

    fn llama_context_settings(&self, n_seq_max: u32) -> LlamaContextSettings<'_> {
        self.loaded_llama_model
            .llama_context_settings(&self.model_runtime_parameters, n_seq_max)
    }
}
