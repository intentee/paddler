use std::num::NonZeroU32;
use std::ops::ControlFlow;
use std::sync::Arc;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use futures_util::FutureExt as _;
use futures_util::TryFutureExt as _;
use futures_util::future::ready;
use tokio_util::sync::CancellationToken;

use paddler_agent::capability_starter::CapabilityStarter;
use paddler_agent::capability_startup_interruption::CapabilityStartupInterruption;
use paddler_agent::loaded_model_serving_plan::LoadedModelServingPlan;
use paddler_agent::model_serving_settings::ModelServingSettings;
use paddler_agent::served_embeddings::ServedEmbeddings;
use paddler_agent::served_model::ServedModel;
use paddler_agent::served_text_generation::ServedTextGeneration;
use paddler_agent_embeddings::embedding_batch_preparer::EmbeddingBatchPreparer;
use paddler_agent_model_capabilities::model_capabilities_settings::ModelCapabilitiesSettings;
use paddler_agent_model_capabilities::multimodal_projection::MultimodalProjection;
use paddler_agent_runtime::scheduler_request_preparer::SchedulerRequestPreparer;
use paddler_agent_text_generation::generation_request_preparer::GenerationRequestPreparer;
use paddler_agent_text_generation::text_generation_settings::TextGenerationSettings;
use paddler_inference_parameters::desired_sampling_parameters::DesiredSamplingParameters;
use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_messaging::chat_template_source::ChatTemplateSource;
use paddler_model_card_fixture::qwen3_5_0_8b::qwen3_5_0_8b;

use crate::agent_inference_runtime_context::agent_inference_runtime_context;
use crate::llama_runtime::LlamaRuntime;

pub struct ModelServingTextGenerationAndEmbeddings {
    pub llama_runtime: LlamaRuntime,
    pub model_serving_settings: ModelServingSettings,
    pub served_model: ServedModel,
}

impl ModelServingTextGenerationAndEmbeddings {
    async fn serve(
        agent_shutdown: &CancellationToken,
        llama_runtime: LlamaRuntime,
    ) -> Result<Self, CapabilityStartupInterruption> {
        let model_serving_settings = ModelServingSettings {
            model_capabilities: ModelCapabilitiesSettings {
                chat_template_source: ChatTemplateSource::EmbeddedInModel,
                embedding_pooling: PoolingType::Last,
                multimodal_projection: MultimodalProjection::NotConfigured,
            },
            model_path: llama_runtime
                .loaded_llama_model
                .llama_model_settings
                .model_path
                .clone(),
            model_runtime_parameters: llama_runtime.model_runtime_parameters.clone(),
            text_generation: TextGenerationSettings {
                image_resize_to_fit: NonZeroU32::MIN,
                sampling_parameters: DesiredSamplingParameters::default(),
            },
        };

        match (CapabilityStarter {
            cancellation_token: agent_shutdown,
            inference_runtime_context: &llama_runtime.inference_runtime_context,
        })
        .serve(LoadedModelServingPlan::StartingEveryCapability {
            loaded_llama_model: llama_runtime.loaded_llama_model.clone(),
            model_serving_settings: model_serving_settings.clone(),
        })
        .await
        {
            ControlFlow::Continue(served_model) => Ok(Self {
                llama_runtime,
                model_serving_settings,
                served_model,
            }),
            ControlFlow::Break(serving_interruption) => Err(serving_interruption.interruption),
        }
    }

    /// # Errors
    ///
    /// Returns an error when the served model serves no embeddings.
    pub fn embeddings_request_preparer(
        &self,
    ) -> Result<Arc<SchedulerRequestPreparer<EmbeddingBatchPreparer>>> {
        match &self.served_model.capabilities.embeddings {
            ServedEmbeddings::NotServed => Err(anyhow!("the served model serves no embeddings")),
            ServedEmbeddings::Served(scheduler_handle) => {
                Ok(Arc::clone(&scheduler_handle.request_preparer))
            }
        }
    }

    /// # Errors
    ///
    /// Returns an error when the Qwen3.5 model is not cached or cannot load, and, inside it, the [`CapabilityStartupInterruption`] of text generation or embeddings that cannot start.
    pub async fn start(
        agent_shutdown: &CancellationToken,
    ) -> Result<Result<Self, CapabilityStartupInterruption>> {
        ready(
            qwen3_5_0_8b()
                .cached_path()
                .context("the Qwen3.5 model must be in the Hugging Face cache"),
        )
        .and_then(|model_path| {
            ready(
                LlamaRuntime::load(&model_path, agent_inference_runtime_context(1))
                    .context("the Qwen3.5 model must load"),
            )
        })
        .and_then(|llama_runtime| Self::serve(agent_shutdown, llama_runtime).map(Ok))
        .await
    }

    /// # Errors
    ///
    /// Returns an error when the served model serves no text generation.
    pub fn text_generation_request_preparer(
        &self,
    ) -> Result<Arc<SchedulerRequestPreparer<GenerationRequestPreparer>>> {
        match &self.served_model.capabilities.text_generation {
            ServedTextGeneration::NotServed => {
                Err(anyhow!("the served model serves no text generation"))
            }
            ServedTextGeneration::Served(scheduler_handle) => {
                Ok(Arc::clone(&scheduler_handle.request_preparer))
            }
        }
    }
}
