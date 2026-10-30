use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_agent_runtime::shuts_down::ShutsDown;

use crate::agent_applicable_state::AgentApplicableState;
use crate::applicable_decisions::ApplicableDecisions;
use crate::loaded_model_serving_plan::LoadedModelServingPlan;
use crate::model_serving_settings::ModelServingSettings;
use crate::pipeline_request::PipelineRequest;
use crate::retained_serving::RetainedServing;
use crate::served_model::ServedModel;

pub enum ModelServing {
    Holding(LoadedLlamaModel),
    Idle,
    Serving(Box<ServedModel>),
}

impl ModelServing {
    #[must_use]
    pub fn decisions_to_prepare(
        &self,
        agent_applicable_state: &AgentApplicableState,
    ) -> ApplicableDecisions {
        match (self, agent_applicable_state) {
            (Self::Serving(served_model), AgentApplicableState::WithModel(desired_settings))
                if served_model.keeps_decisions_of(desired_settings) =>
            {
                ApplicableDecisions::Disabled
            }
            (_, AgentApplicableState::WithModel(desired_settings)) => {
                desired_settings.capabilities.decisions.clone()
            }
            (
                _,
                AgentApplicableState::NotConfigured | AgentApplicableState::WithoutModel { .. },
            ) => ApplicableDecisions::Disabled,
        }
    }

    pub fn forward(&self, agent_name: Option<&str>, request: PipelineRequest) {
        match self {
            Self::Holding(_) | Self::Idle => request.reject_because_no_model_is_loaded(agent_name),
            Self::Serving(served_model) => served_model.forward(agent_name, request),
        }
    }

    pub async fn retain_for(self, desired_settings: ModelServingSettings) -> RetainedServing {
        match self {
            Self::Holding(loaded_llama_model)
                if loaded_llama_model.llama_model_settings
                    == desired_settings.llama_model_settings() =>
            {
                RetainedServing::LoadedModel(LoadedModelServingPlan::starting_every_capability(
                    loaded_llama_model,
                    desired_settings,
                ))
            }
            Self::Holding(loaded_llama_model) => {
                loaded_llama_model.release().await;

                RetainedServing::ModelToLoad(desired_settings)
            }
            Self::Idle => RetainedServing::ModelToLoad(desired_settings),
            Self::Serving(served_model) => served_model.retain_for(desired_settings).await,
        }
    }
}

impl ShutsDown for ModelServing {
    async fn shut_down(self) {
        match self {
            Self::Holding(loaded_llama_model) => loaded_llama_model.release().await,
            Self::Idle => {}
            Self::Serving(served_model) => served_model.shut_down().await,
        }
    }
}
