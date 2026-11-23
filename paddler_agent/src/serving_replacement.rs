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
use crate::prepared_decisions::PreparedDecisions;
use crate::prepared_serving::PreparedServing;
use crate::replacement_outcome::ReplacementOutcome;
use crate::retained_serving::RetainedServing;
use crate::serving_interruption::ServingInterruption;

pub struct ServingReplacement {
    pub held: ModelServing,
    pub prepared_serving: PreparedServing,
}

impl ServingReplacement {
    pub async fn replace(
        self,
        cancellation_token: CancellationToken,
        inference_runtime_context: InferenceRuntimeContext,
    ) -> FinishedServingStartup {
        let Self {
            held,
            prepared_serving:
                PreparedServing {
                    agent_applicable_state,
                    prepared_decisions,
                },
        } = self;
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
                Self::serve(
                    &capability_starter,
                    held,
                    desired_settings.clone(),
                    prepared_decisions,
                )
                .await
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
        prepared_decisions: PreparedDecisions,
    ) -> ReplacementOutcome {
        let loaded_model_serving_plan = match held.retain_for(desired_settings).await {
            RetainedServing::LoadedModel(loaded_model_serving_plan) => loaded_model_serving_plan,
            RetainedServing::ModelToLoad(desired_settings) => match capability_starter
                .load_model(desired_settings.llama_model_settings())
                .await
            {
                ControlFlow::Continue(loaded_llama_model) => {
                    LoadedModelServingPlan::starting_every_capability(
                        loaded_llama_model,
                        desired_settings,
                    )
                }
                ControlFlow::Break(interruption) => {
                    return interruption.into_replacement_outcome(ModelServing::Idle);
                }
            },
        };

        match capability_starter
            .serve(loaded_model_serving_plan, prepared_decisions)
            .await
        {
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
