use std::path::Path;

use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_decision::decision_llama_context_settings::DecisionLlamaContextSettings;
use paddler_agent_embeddings::embedding_llama_context_settings::EmbeddingLlamaContextSettings;
use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_runtime::llama_context_settings::LlamaContextSettings;
use paddler_agent_runtime::llama_model_settings::LlamaModelSettings;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_agent_runtime::resident_llama_context::ResidentLlamaContext;
use paddler_agent_text_generation::sampler_chain_factory::SamplerChainFactory;
use paddler_inference_parameters::all_gpu_layers::ALL_GPU_LAYERS;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_inference_parameters::sampling_parameters::SamplingParameters;
use paddler_messaging::inference_capability::InferenceCapability;

pub struct LlamaRuntime {
    pub inference_runtime_context: InferenceRuntimeContext,
    pub loaded_llama_model: LoadedLlamaModel,
    pub model_runtime_parameters: ModelRuntimeParameters,
}

impl LlamaRuntime {
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

    pub fn create_decision_llama_context(
        &self,
        n_seq_max: u32,
    ) -> Result<ResidentLlamaContext<'_>, DecisionError> {
        DecisionLlamaContextSettings {
            llama_context_settings: self.llama_context_settings(n_seq_max),
        }
        .create_llama_context(&self.loaded_llama_model, &self.inference_runtime_context)
    }

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
