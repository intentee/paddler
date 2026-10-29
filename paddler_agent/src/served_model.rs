use tokio::join;

use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_agent_runtime::shuts_down::ShutsDown;
use paddler_agent_status::serving_report::ServingReport;
use paddler_agent_text_generation::text_generation_request::TextGenerationRequest;

use crate::capability_plan::CapabilityPlan;
use crate::inference_capabilities::InferenceCapabilities;
use crate::loaded_model_serving_plan::LoadedModelServingPlan;
use crate::model_serving_settings::ModelServingSettings;
use crate::pipeline_request::PipelineRequest;
use crate::retained_serving::RetainedServing;
use crate::served_capabilities::ServedCapabilities;

pub struct ServedModel {
    pub capabilities: ServedCapabilities,
    pub loaded_llama_model: LoadedLlamaModel,
    pub model_serving_settings: ModelServingSettings,
}

impl ServedModel {
    pub fn forward(&self, agent_name: Option<&str>, request: PipelineRequest) {
        match request {
            PipelineRequest::ContinueFromConversationHistory(request) => {
                self.capabilities.text_generation.request_preparer.prepare(
                    TextGenerationRequest::ContinueFromConversationHistory(request),
                );
            }
            PipelineRequest::ContinueFromRawPrompt(request) => {
                self.capabilities
                    .text_generation
                    .request_preparer
                    .prepare(TextGenerationRequest::ContinueFromRawPrompt(request));
            }
            PipelineRequest::Decide(request) => {
                self.capabilities.decisions.forward(agent_name, request);
            }
            PipelineRequest::GenerateEmbeddingBatch(request) => {
                self.capabilities
                    .embeddings
                    .request_preparer
                    .prepare(request);
            }
        }
    }

    #[must_use]
    pub fn keeps_decisions_of(&self, desired_settings: &ModelServingSettings) -> bool {
        self.serves_the_model_of(desired_settings)
            && self.runs_the_runtime_parameters_of(desired_settings)
            && self.model_serving_settings.capabilities.decisions
                == desired_settings.capabilities.decisions
    }

    pub async fn retain_for(self, desired_settings: ModelServingSettings) -> RetainedServing {
        let serves_the_desired_model = self.serves_the_model_of(&desired_settings);
        let runs_the_desired_runtime_parameters =
            self.runs_the_runtime_parameters_of(&desired_settings);
        let Self {
            capabilities,
            loaded_llama_model,
            model_serving_settings: served_settings,
        } = self;

        if !serves_the_desired_model {
            capabilities.shut_down().await;
            loaded_llama_model.release().await;

            return RetainedServing::ModelToLoad(desired_settings);
        }

        if !runs_the_desired_runtime_parameters {
            capabilities.shut_down().await;

            return RetainedServing::LoadedModel(
                LoadedModelServingPlan::starting_every_capability(
                    loaded_llama_model,
                    desired_settings,
                ),
            );
        }

        let InferenceCapabilities {
            decisions,
            embeddings,
            text_generation,
        } = capabilities;

        let (decisions_plan, embeddings_plan, text_generation_plan) = join!(
            CapabilityPlan::retaining(
                decisions,
                &served_settings.capabilities.decisions,
                &desired_settings.capabilities.decisions,
            ),
            CapabilityPlan::retaining(
                embeddings,
                &served_settings.capabilities.embeddings,
                &desired_settings.capabilities.embeddings,
            ),
            CapabilityPlan::retaining(
                text_generation,
                &served_settings.capabilities.text_generation,
                &desired_settings.capabilities.text_generation,
            ),
        );

        RetainedServing::LoadedModel(LoadedModelServingPlan {
            capability_plans: InferenceCapabilities {
                decisions: decisions_plan,
                embeddings: embeddings_plan,
                text_generation: text_generation_plan,
            },
            loaded_llama_model,
            model_serving_settings: desired_settings,
        })
    }

    fn serves_the_model_of(&self, desired_settings: &ModelServingSettings) -> bool {
        self.loaded_llama_model.llama_model_settings == desired_settings.llama_model_settings()
    }

    fn runs_the_runtime_parameters_of(&self, desired_settings: &ModelServingSettings) -> bool {
        self.model_serving_settings.model_runtime_parameters
            == desired_settings.model_runtime_parameters
    }

    #[must_use]
    pub fn serving_report(&self, uses_chat_template_override: bool) -> ServingReport {
        ServingReport {
            decisions: self.model_serving_settings.capabilities.decisions.serving(),
            model_path: self
                .loaded_llama_model
                .llama_model_settings
                .reported_model_path()
                .model_path,
            serving_issues: self
                .capabilities
                .text_generation
                .request_preparer
                .preparation
                .conversation_support
                .serving_issues(),
            uses_chat_template_override,
        }
    }
}

impl ShutsDown for ServedModel {
    async fn shut_down(self) {
        let Self {
            capabilities,
            loaded_llama_model,
            ..
        } = self;

        capabilities.shut_down().await;
        loaded_llama_model.release().await;
    }
}
