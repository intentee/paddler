use tokio_util::sync::CancellationToken;

use paddler_agent_decision::decision_pipeline::DecisionPipeline;
use paddler_agent_decision::decision_prerequisites::DecisionPrerequisites;
use paddler_agent_embeddings::embedding_pipeline::EmbeddingPipeline;
use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_runtime::llama_model_settings::LlamaModelSettings;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_agent_runtime::shuts_down::ShutsDown as _;
use paddler_agent_runtime::startup_outcome::StartupOutcome;
use paddler_agent_text_generation::text_generation_pipeline::TextGenerationPipeline;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_spawn_resuming_panic::spawn_blocking_resuming_panic::spawn_blocking_resuming_panic;

use crate::agent_error::AgentError;
use crate::applicable_decisions::ApplicableDecisions;
use crate::capability_plan::CapabilityPlan;
use crate::capability_startup_interruption::CapabilityStartupInterruption;
use crate::inference_capabilities::InferenceCapabilities;
use crate::loaded_model_serving_plan::LoadedModelServingPlan;
use crate::model_serving_settings::ModelServingSettings;
use crate::prepared_decisions::PreparedDecisions;
use crate::served_capabilities::ServedCapabilities;
use crate::served_decisions::ServedDecisions;
use crate::served_embeddings::ServedEmbeddings;
use crate::served_model::ServedModel;
use crate::served_text_generation::ServedTextGeneration;
use crate::serving_interruption::ServingInterruption;

trait IntoEstablishedCapability<TStarted> {
    fn into_established_capability(self) -> Result<TStarted, CapabilityStartupInterruption>;
}

impl<TStarted, TError> IntoEstablishedCapability<TStarted>
    for Result<StartupOutcome<TStarted>, TError>
where
    AgentError: From<TError>,
{
    fn into_established_capability(self) -> Result<TStarted, CapabilityStartupInterruption> {
        match self {
            Ok(StartupOutcome::Ready(started)) => Ok(started),
            Ok(StartupOutcome::Cancelled) => Err(CapabilityStartupInterruption::Cancelled),
            Err(startup_error) => Err(CapabilityStartupInterruption::Failed(AgentError::from(
                startup_error,
            ))),
        }
    }
}

pub struct CapabilityStarter<'startup> {
    pub cancellation_token: &'startup CancellationToken,
    pub inference_runtime_context: &'startup InferenceRuntimeContext,
}

impl CapabilityStarter<'_> {
    pub async fn prepare_decisions(
        &self,
        applicable_decisions: &ApplicableDecisions,
    ) -> Result<PreparedDecisions, CapabilityStartupInterruption> {
        match applicable_decisions {
            ApplicableDecisions::Disabled => Ok(PreparedDecisions::Disabled),
            ApplicableDecisions::Enabled { pointer_head_path } => {
                let decision_prerequisites = DecisionPrerequisites {
                    inference_runtime_context: self.inference_runtime_context.clone(),
                    pointer_head_path: pointer_head_path.clone(),
                };
                let cancellation_token = self.cancellation_token.clone();

                spawn_blocking_resuming_panic(move || {
                    decision_prerequisites
                        .load_pointer_head()
                        .map(|loaded_pointer_head| {
                            if cancellation_token.is_cancelled() {
                                StartupOutcome::Cancelled
                            } else {
                                StartupOutcome::Ready(loaded_pointer_head)
                            }
                        })
                })
                .await
                .into_established_capability()
                .map(|loaded_pointer_head| {
                    PreparedDecisions::Enabled(Box::new(loaded_pointer_head))
                })
            }
        }
    }

    pub async fn load_model(
        &self,
        llama_model_settings: LlamaModelSettings,
    ) -> Result<LoadedLlamaModel, CapabilityStartupInterruption> {
        LoadedLlamaModel::load_unless_shutting_down(
            self.cancellation_token,
            self.inference_runtime_context.clone(),
            llama_model_settings,
        )
        .await
        .into_established_capability()
    }

    pub async fn serve(
        &self,
        LoadedModelServingPlan {
            capability_plans:
                InferenceCapabilities {
                    decisions: decisions_plan,
                    embeddings: embeddings_plan,
                    text_generation: text_generation_plan,
                },
            loaded_llama_model,
            model_serving_settings,
        }: LoadedModelServingPlan,
        prepared_decisions: PreparedDecisions,
    ) -> Result<ServedModel, Box<ServingInterruption>> {
        let decisions = match self
            .establish_decisions(
                &loaded_llama_model,
                &model_serving_settings.model_runtime_parameters,
                decisions_plan,
                prepared_decisions,
            )
            .await
        {
            Ok(decisions) => decisions,
            Err(interruption) => {
                embeddings_plan.shut_down().await;
                text_generation_plan.shut_down().await;

                return Err(Box::new(ServingInterruption {
                    interruption,
                    loaded_llama_model,
                }));
            }
        };
        let embeddings = match self
            .establish_embeddings(
                &loaded_llama_model,
                &model_serving_settings,
                embeddings_plan,
            )
            .await
        {
            Ok(embeddings) => embeddings,
            Err(interruption) => {
                decisions.shut_down().await;
                text_generation_plan.shut_down().await;

                return Err(Box::new(ServingInterruption {
                    interruption,
                    loaded_llama_model,
                }));
            }
        };
        let text_generation = match self
            .establish_text_generation(
                &loaded_llama_model,
                &model_serving_settings,
                text_generation_plan,
            )
            .await
        {
            Ok(text_generation) => text_generation,
            Err(interruption) => {
                decisions.shut_down().await;
                embeddings.shut_down().await;

                return Err(Box::new(ServingInterruption {
                    interruption,
                    loaded_llama_model,
                }));
            }
        };

        Ok(ServedModel {
            capabilities: ServedCapabilities {
                decisions,
                embeddings,
                text_generation,
            },
            loaded_llama_model,
            model_serving_settings,
        })
    }

    async fn establish_decisions(
        &self,
        loaded_llama_model: &LoadedLlamaModel,
        model_runtime_parameters: &ModelRuntimeParameters,
        decisions_plan: CapabilityPlan<ServedDecisions>,
        prepared_decisions: PreparedDecisions,
    ) -> Result<ServedDecisions, CapabilityStartupInterruption> {
        match (decisions_plan, prepared_decisions) {
            (CapabilityPlan::Keep(served_decisions), _) => Ok(served_decisions),
            (CapabilityPlan::Start, PreparedDecisions::Disabled) => Ok(ServedDecisions::Disabled),
            (CapabilityPlan::Start, PreparedDecisions::Enabled(loaded_pointer_head)) => {
                DecisionPipeline {
                    inference_runtime_context: self.inference_runtime_context.clone(),
                    loaded_llama_model: loaded_llama_model.clone(),
                    loaded_pointer_head: *loaded_pointer_head,
                    model_runtime_parameters: model_runtime_parameters.clone(),
                }
                .spawn(self.cancellation_token)
                .await
                .into_established_capability()
                .map(ServedDecisions::Enabled)
            }
        }
    }

    async fn establish_embeddings(
        &self,
        loaded_llama_model: &LoadedLlamaModel,
        model_serving_settings: &ModelServingSettings,
        embeddings_plan: CapabilityPlan<ServedEmbeddings>,
    ) -> Result<ServedEmbeddings, CapabilityStartupInterruption> {
        match embeddings_plan {
            CapabilityPlan::Keep(embeddings) => Ok(embeddings),
            CapabilityPlan::Start => EmbeddingPipeline {
                inference_runtime_context: self.inference_runtime_context.clone(),
                loaded_llama_model: loaded_llama_model.clone(),
                model_runtime_parameters: model_serving_settings.model_runtime_parameters.clone(),
                pooling_type: model_serving_settings.capabilities.embeddings.clone(),
            }
            .spawn(self.cancellation_token)
            .await
            .into_established_capability(),
        }
    }

    async fn establish_text_generation(
        &self,
        loaded_llama_model: &LoadedLlamaModel,
        model_serving_settings: &ModelServingSettings,
        text_generation_plan: CapabilityPlan<ServedTextGeneration>,
    ) -> Result<ServedTextGeneration, CapabilityStartupInterruption> {
        match text_generation_plan {
            CapabilityPlan::Keep(text_generation) => Ok(text_generation),
            CapabilityPlan::Start => TextGenerationPipeline {
                inference_runtime_context: self.inference_runtime_context.clone(),
                loaded_llama_model: loaded_llama_model.clone(),
                model_runtime_parameters: model_serving_settings.model_runtime_parameters.clone(),
                text_generation_settings: model_serving_settings
                    .capabilities
                    .text_generation
                    .clone(),
            }
            .spawn(self.cancellation_token)
            .await
            .into_established_capability(),
        }
    }
}
