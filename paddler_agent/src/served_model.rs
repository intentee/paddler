use std::sync::Arc;

use paddler_agent_model_capabilities::established_model_capabilities::EstablishedModelCapabilities;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_agent_runtime::shuts_down::ShutsDown;
use paddler_agent_status::serving_report::ServingReport;
use paddler_agent_text_generation::text_generation_request::TextGenerationRequest;

use crate::capability_plan::CapabilityPlan;
use crate::established_capabilities_plan::EstablishedCapabilitiesPlan;
use crate::inference_capabilities::InferenceCapabilities;
use crate::loaded_model_serving_plan::LoadedModelServingPlan;
use crate::model_serving_settings::ModelServingSettings;
use crate::pipeline_request::PipelineRequest;
use crate::retained_serving::RetainedServing;
use crate::served_capabilities::ServedCapabilities;

pub struct ServedModel {
    pub capabilities: ServedCapabilities,
    pub loaded_llama_model: LoadedLlamaModel,
    pub model_capabilities: Arc<EstablishedModelCapabilities>,
    pub model_serving_settings: ModelServingSettings,
}

impl ServedModel {
    pub fn forward(&self, agent_name: Option<&str>, request: PipelineRequest) {
        match request {
            PipelineRequest::ContinueFromConversationHistory(request) => {
                self.capabilities.text_generation.forward(
                    agent_name,
                    TextGenerationRequest::ContinueFromConversationHistory(request),
                );
            }
            PipelineRequest::ContinueFromRawPrompt(request) => {
                self.capabilities.text_generation.forward(
                    agent_name,
                    TextGenerationRequest::ContinueFromRawPrompt(request),
                );
            }
            PipelineRequest::Decide(request) => {
                self.capabilities.decisions.forward(agent_name, request);
            }
            PipelineRequest::GenerateEmbeddingBatch(request) => {
                self.capabilities.embeddings.forward(agent_name, request);
            }
        }
    }

    pub async fn retain_for(self, desired_settings: ModelServingSettings) -> RetainedServing {
        let serves_the_desired_model = self.serves_the_model_of(&desired_settings);
        let keeps_its_capabilities = self.keeps_the_capabilities_of(&desired_settings);
        let Self {
            capabilities,
            loaded_llama_model,
            model_capabilities,
            model_serving_settings: served_settings,
        } = self;

        if !serves_the_desired_model {
            capabilities.shut_down().await;
            drop(model_capabilities);
            loaded_llama_model.release().await;

            return RetainedServing::ModelToLoad(desired_settings);
        }

        if !keeps_its_capabilities {
            capabilities.shut_down().await;
            drop(model_capabilities);

            return RetainedServing::LoadedModel(Box::new(
                LoadedModelServingPlan::starting_every_capability(
                    loaded_llama_model,
                    desired_settings,
                ),
            ));
        }

        let InferenceCapabilities {
            decisions,
            embeddings,
            text_generation,
        } = capabilities;

        RetainedServing::LoadedModel(Box::new(LoadedModelServingPlan {
            capability_plans: InferenceCapabilities {
                decisions: CapabilityPlan::Keep(decisions),
                embeddings: CapabilityPlan::Keep(embeddings),
                text_generation: CapabilityPlan::retaining(
                    text_generation,
                    &served_settings.text_generation,
                    &desired_settings.text_generation,
                )
                .await,
            },
            established_capabilities: EstablishedCapabilitiesPlan::Keep(model_capabilities),
            loaded_llama_model,
            model_serving_settings: desired_settings,
        }))
    }

    #[must_use]
    pub fn serving_report(&self, uses_chat_template_override: bool) -> ServingReport {
        ServingReport {
            capabilities: self.model_capabilities.reported,
            model_path: self
                .loaded_llama_model
                .llama_model_settings
                .reported_model_path()
                .model_path,
            uses_chat_template_override,
        }
    }

    fn keeps_the_capabilities_of(
        &self,
        ModelServingSettings {
            model_capabilities,
            model_runtime_parameters,
            ..
        }: &ModelServingSettings,
    ) -> bool {
        self.model_serving_settings.model_runtime_parameters == *model_runtime_parameters
            && self.model_serving_settings.model_capabilities == *model_capabilities
    }

    fn serves_the_model_of(&self, desired_settings: &ModelServingSettings) -> bool {
        self.loaded_llama_model.llama_model_settings == desired_settings.llama_model_settings()
    }
}

impl ShutsDown for ServedModel {
    async fn shut_down(self) {
        let Self {
            capabilities,
            loaded_llama_model,
            model_capabilities,
            ..
        } = self;

        capabilities.shut_down().await;
        drop(model_capabilities);
        loaded_llama_model.release().await;
    }
}
