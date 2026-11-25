use std::ops::ControlFlow;

use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_runtime::shuts_down::ShutsDown as _;

use crate::agent_applicable_state::AgentApplicableState;
use crate::capability_starter::CapabilityStarter;
use crate::finished_serving_startup::FinishedServingStartup;
use crate::loaded_model_serving_plan::LoadedModelServingPlan;
use crate::model_serving::ModelServing;
use crate::model_serving_settings::ModelServingSettings;
use crate::replacement_outcome::ReplacementOutcome;
use crate::retained_serving::RetainedServing;
use crate::retry_delay::RetryDelay;
use crate::retry_delay_outcome::RetryDelayOutcome;
use crate::serving_interruption::ServingInterruption;

pub struct ServingReplacement {
    pub agent_applicable_state: AgentApplicableState,
    pub held: ModelServing,
    pub retry_delay: RetryDelay,
}

impl ServingReplacement {
    pub async fn replace(
        self,
        cancellation_token: CancellationToken,
        inference_runtime_context: InferenceRuntimeContext,
    ) -> FinishedServingStartup {
        let Self {
            agent_applicable_state,
            held,
            retry_delay,
        } = self;

        if retry_delay.elapse(&cancellation_token).await == RetryDelayOutcome::Cancelled {
            return FinishedServingStartup {
                agent_applicable_state,
                outcome: ReplacementOutcome::Cancelled { held },
            };
        }

        let capability_starter = CapabilityStarter {
            cancellation_token: &cancellation_token,
            inference_runtime_context: &inference_runtime_context,
        };
        let outcome = match &agent_applicable_state {
            AgentApplicableState::NotConfigured | AgentApplicableState::WithoutModel { .. } => {
                held.shut_down().await;

                ReplacementOutcome::NothingToServe
            }
            AgentApplicableState::WithModel(desired_settings) => {
                Self::serve(&capability_starter, held, desired_settings.clone()).await
            }
        };

        FinishedServingStartup {
            agent_applicable_state,
            outcome,
        }
    }

    async fn serve(
        capability_starter: &CapabilityStarter<'_>,
        held: ModelServing,
        desired_settings: ModelServingSettings,
    ) -> ReplacementOutcome {
        let loaded_model_serving_plan = match held.retain_for(desired_settings).await {
            RetainedServing::LoadedModel(loaded_model_serving_plan) => *loaded_model_serving_plan,
            RetainedServing::ModelToLoad(desired_settings) => match capability_starter
                .load_model(desired_settings.llama_model_settings())
                .await
            {
                ControlFlow::Continue(loaded_llama_model) => {
                    LoadedModelServingPlan::StartingEveryCapability {
                        loaded_llama_model: loaded_llama_model,
                        model_serving_settings: desired_settings,
                    }
                }
                ControlFlow::Break(interruption) => {
                    return interruption.into_replacement_outcome(ModelServing::Idle);
                }
            },
        };

        match capability_starter.serve(loaded_model_serving_plan).await {
            ControlFlow::Continue(served_model) => {
                ReplacementOutcome::Serving(Box::new(served_model))
            }
            ControlFlow::Break(serving_interruption) => {
                let ServingInterruption {
                    interruption,
                    loaded_llama_model,
                } = *serving_interruption;

                interruption.into_replacement_outcome(ModelServing::Holding(loaded_llama_model))
            }
        }
    }
}
