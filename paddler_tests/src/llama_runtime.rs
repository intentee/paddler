use std::path::Path;

use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::model::params::LlamaModelParams;

use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_decision::decision_llama_context_settings::DecisionLlamaContextSettings;
use paddler_agent_embeddings::embedding_llama_context_settings::EmbeddingLlamaContextSettings;
use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_runtime::inference_thread_count::inference_thread_count;
use paddler_agent_runtime::llama_context_settings::LlamaContextSettings;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_agent_text_generation::chat_prompt_renderer::ChatPromptRenderer;
use paddler_agent_text_generation::generation_request_preparer::GenerationRequestPreparer;
use paddler_agent_text_generation::image_input::ImageInput;
use paddler_agent_text_generation::model_bos_token::ModelBosToken;
use paddler_agent_text_generation::prompt_tokenizer::PromptTokenizer;
use paddler_agent_text_generation::sampler_chain_factory::SamplerChainFactory;
use paddler_agent_text_generation::token_generation_support::TokenGenerationSupport;
use paddler_inference_parameters::all_gpu_layers::ALL_GPU_LAYERS;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_inference_parameters::sampling_parameters::SamplingParameters;
use paddler_messaging::media_marker::MediaMarker;
use paddler_messaging::multimodal_settings::MultimodalSettings;

use crate::text_only_generation_parts::TextOnlyGenerationParts;

pub struct LlamaRuntime {
    pub inference_runtime_context: InferenceRuntimeContext,
    pub loaded_llama_model: LoadedLlamaModel,
    pub model_runtime_parameters: ModelRuntimeParameters,
    pub thread_count: i32,
}

impl LlamaRuntime {
    pub fn load(
        model_path: &Path,
        inference_runtime_context: InferenceRuntimeContext,
    ) -> Result<Self, AgentRuntimeError> {
        let thread_count =
            inference_thread_count(inference_runtime_context.available_parallelism.as_ref())?;
        let model_runtime_parameters = ModelRuntimeParameters {
            n_gpu_layers: ALL_GPU_LAYERS,
            ..ModelRuntimeParameters::default()
        };
        let loaded_llama_model = LoadedLlamaModel::load(
            &inference_runtime_context,
            model_path,
            &LlamaModelParams::default().with_n_gpu_layers(model_runtime_parameters.n_gpu_layers),
        )?;

        Ok(Self {
            inference_runtime_context,
            loaded_llama_model,
            model_runtime_parameters,
            thread_count,
        })
    }

    pub fn create_decision_llama_context(
        &self,
        n_seq_max: u32,
    ) -> Result<LlamaContext<'_>, DecisionError> {
        DecisionLlamaContextSettings {
            llama_context_settings: self.llama_context_settings(n_seq_max),
        }
        .create_llama_context(&self.loaded_llama_model, &self.inference_runtime_context)
    }

    pub fn create_embedding_llama_context(
        &self,
        n_seq_max: u32,
        pooling_type: PoolingType,
    ) -> Result<LlamaContext<'_>, AgentRuntimeError> {
        EmbeddingLlamaContextSettings {
            llama_context_settings: self.llama_context_settings(n_seq_max),
            pooling_type,
        }
        .create_llama_context(&self.loaded_llama_model, &self.inference_runtime_context)
    }

    pub fn create_text_generation_llama_context(
        &self,
        n_seq_max: u32,
    ) -> Result<LlamaContext<'_>, AgentRuntimeError> {
        self.loaded_llama_model.create_llama_context(
            &self.inference_runtime_context,
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

    #[must_use]
    pub fn text_only_generation_request_preparer<TSamplerChainFactory, TToolsSerializer>(
        &self,
        TextOnlyGenerationParts {
            chat_template_renderer,
            sequence_context_size,
            streaming_markers,
        }: TextOnlyGenerationParts,
        sampler_chain_factory: TSamplerChainFactory,
        tools_serializer: TToolsSerializer,
    ) -> GenerationRequestPreparer<TSamplerChainFactory, TToolsSerializer> {
        let model = &self.loaded_llama_model.model;

        GenerationRequestPreparer {
            image_input: ImageInput::Unsupported,
            image_resize_to_fit: MultimodalSettings::default().image_resize_to_fit,
            loaded_llama_model: self.loaded_llama_model.clone(),
            prompt_tokenizer: PromptTokenizer {
                model: model.clone(),
                model_bos_token: ModelBosToken::of(model),
                sequence_context_size,
            },
            sampler_chain_factory,
            token_generation_support: TokenGenerationSupport {
                chat_prompt_renderer: ChatPromptRenderer {
                    chat_template_renderer,
                    media_marker: MediaMarker::new("<media>".to_owned()),
                    token_bos_str: String::new(),
                    token_eos_str: String::new(),
                    token_nl_str: String::new(),
                },
                streaming_markers,
            },
            tools_serializer,
        }
    }

    const fn llama_context_settings(&self, n_seq_max: u32) -> LlamaContextSettings<'_> {
        LlamaContextSettings {
            model_runtime_parameters: &self.model_runtime_parameters,
            n_seq_max,
            thread_count: self.thread_count,
        }
    }
}
