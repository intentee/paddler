use std::path::PathBuf;

use futures::executor::block_on;
use hf_hub::HFResult;

use paddler_inference_parameters::constants::ALL_GPU_LAYERS;
use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_inference_parameters::sampling_parameters::SamplingParameters;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::balancer_text_generation_settings::BalancerTextGenerationSettings;
use paddler_messaging::multimodal_settings::MultimodalSettings;
use paddler_model_source::huggingface_cache_location::HuggingFaceCacheLocation;
use paddler_model_source::huggingface_model_reference::HuggingFaceModelReference;
use paddler_model_source::model_source::ModelSource;

pub struct ModelCard {
    pub reference: HuggingFaceModelReference,
}

impl ModelCard {
    /// # Errors
    ///
    /// Returns the [`hf_hub::HFError`] of a Hugging Face client that cannot be built, [`hf_hub::HFError::LocalEntryNotFound`] or [`hf_hub::HFError::EntryNotFound`] when the model file is not in the local Hugging Face cache, and another [`hf_hub::HFError`] when the cache cannot be read.
    pub fn cached_path(&self) -> HFResult<PathBuf> {
        HuggingFaceCacheLocation::HubDefault
            .client_builder()
            .build()
            .and_then(|client| {
                block_on(
                    self.reference
                        .cached_path(&self.reference.repository(&client)),
                )
            })
    }

    #[must_use]
    pub fn into_agent_desired_model(self) -> AgentDesiredModel {
        ModelSource::HuggingFace(self.reference).into_agent_desired_model()
    }

    #[must_use]
    pub fn into_desired_state(self) -> BalancerDesiredState {
        BalancerDesiredState {
            model: self.into_agent_desired_model(),
            model_runtime_parameters: ModelRuntimeParameters {
                n_gpu_layers: ALL_GPU_LAYERS,
                ..ModelRuntimeParameters::default()
            },
            text_generation: BalancerTextGenerationSettings {
                sampling_parameters: SamplingParameters::deterministic(),
                ..BalancerTextGenerationSettings::default()
            },
            ..BalancerDesiredState::default()
        }
    }

    #[must_use]
    pub fn into_desired_state_with_embedding_parameters(
        self,
        embedding_parameters: EmbeddingParameters,
    ) -> BalancerDesiredState {
        BalancerDesiredState {
            embeddings: embedding_parameters,
            ..self.into_desired_state()
        }
    }

    #[must_use]
    pub fn into_desired_state_with_multimodal_projection(
        self,
        multimodal_projection: Self,
    ) -> BalancerDesiredState {
        let desired_state = self.into_desired_state();

        BalancerDesiredState {
            text_generation: BalancerTextGenerationSettings {
                multimodal: MultimodalSettings {
                    projection: multimodal_projection.into_agent_desired_model(),
                    ..MultimodalSettings::default()
                },
                ..desired_state.text_generation
            },
            ..desired_state
        }
    }
}
