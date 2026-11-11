use std::path::Path;
use std::sync::Arc;

use anyhow::Context as _;
use anyhow::Result as AnyhowResult;
use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::context::context_limits::ContextLimits;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::model::add_bos::AddBos;
use llama_cpp_bindings::model::parse_special_tokens::ParseSpecialTokens;

use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_decision::decision_llama_context_settings::DecisionLlamaContextSettings;
use paddler_agent_embeddings::embedding_llama_context_settings::EmbeddingLlamaContextSettings;
use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_runtime::llama_context_settings::LlamaContextSettings;
use paddler_agent_runtime::llama_model_settings::LlamaModelSettings;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_agent_runtime::resident_llama_context::ResidentLlamaContext;
use paddler_agent_text_generation::constants::LAST_LOGITS_BATCH_POSITION;
use paddler_agent_text_generation::multimodal_prompt_support::MultimodalPromptSupport;
use paddler_agent_text_generation::resident_multimodal_context::ResidentMultimodalContext;
use paddler_agent_text_generation::sampler_chain_factory::SamplerChainFactory;
use paddler_inference_parameters::constants::ALL_GPU_LAYERS;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_inference_parameters::sampling_parameters::SamplingParameters;
use paddler_messaging::inference_capability::InferenceCapability;

use crate::prompt_sampling_positions::PromptSamplingPositions;

pub struct LlamaRuntime {
    pub inference_runtime_context: InferenceRuntimeContext,
    pub loaded_llama_model: LoadedLlamaModel,
    pub model_runtime_parameters: ModelRuntimeParameters,
}

impl LlamaRuntime {
    /// # Errors
    ///
    /// Returns the [`AgentRuntimeError`] of [`LoadedLlamaModel::load`] when the model at `model_path` cannot be loaded.
    pub fn load(
        model_path: &Path,
        inference_runtime_context: InferenceRuntimeContext,
    ) -> Result<Self, AgentRuntimeError> {
        let model_runtime_parameters = ModelRuntimeParameters {
            n_gpu_layers: ALL_GPU_LAYERS,
            ..ModelRuntimeParameters::default()
        };
        let loaded_llama_model = LoadedLlamaModel::load(
            &inference_runtime_context,
            LlamaModelSettings {
                model_path: model_path.to_path_buf(),
                n_gpu_layers: model_runtime_parameters.n_gpu_layers,
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
        n_seq_max: u32,
    ) -> Result<ResidentLlamaContext<'_>, DecisionError> {
        DecisionLlamaContextSettings {
            llama_context_settings: self.llama_context_settings(n_seq_max),
        }
        .create_llama_context(&self.loaded_llama_model, &self.inference_runtime_context)
    }

    /// # Errors
    ///
    /// Returns [`AgentRuntimeError::LlamaContextAlreadyResident`] when an embedding context already occupies the VRAM, and [`AgentRuntimeError::ContextCreationFailed`] when llama.cpp cannot create the context.
    pub fn create_embedding_llama_context(
        &self,
        n_seq_max: u32,
        pooling_type: PoolingType,
    ) -> Result<ResidentLlamaContext<'_>, AgentRuntimeError> {
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
                let mut batch = LlamaBatch::new(ContextLimits::of(llama_context));

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
    /// Returns an error when another multimodal projection is resident, and when the multimodal projection at `multimodal_projection_path` cannot load.
    pub fn multimodal_prompt_support(
        &self,
        llama_context: &LlamaContext<'_>,
        multimodal_projection_path: &Path,
    ) -> AnyhowResult<MultimodalPromptSupport> {
        let n_batch = self.model_runtime_parameters.n_batch;

        self.inference_runtime_context
            .vram_residencies
            .multimodal_projection
            .try_occupy()
            .context("no other multimodal projection may be resident")
            .and_then(|vram_occupancy| {
                ResidentMultimodalContext::load(
                    multimodal_projection_path,
                    &self.loaded_llama_model,
                    vram_occupancy,
                )
                .context("the multimodal projection must load")
            })
            .map(|multimodal_context| MultimodalPromptSupport {
                micro_batch_tokens: ContextLimits::of(llama_context)
                    .micro_batch_tokens(n_batch.tokens()),
                multimodal_context: Arc::new(multimodal_context),
                n_batch: n_batch.tokens_i32(),
                sequence_context_size: llama_context.n_ctx_seq(),
            })
    }

    #[must_use]
    pub fn sampler_chain_factory(&self) -> SamplerChainFactory {
        SamplerChainFactory {
            sampling_parameters: SamplingParameters::default(),
            n_vocab: self.loaded_llama_model.model.n_vocab(),
        }
    }

    const fn llama_context_settings(&self, n_seq_max: u32) -> LlamaContextSettings<'_> {
        self.loaded_llama_model
            .llama_context_settings(&self.model_runtime_parameters, n_seq_max)
    }
}
