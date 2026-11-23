use std::ops::ControlFlow;

use tokio_util::sync::CancellationToken;

use paddler_agent_decision::decision_pipeline::DecisionPipeline;
use paddler_agent_decision::decision_prerequisites::DecisionPrerequisites;
use paddler_agent_embeddings::embedding_pipeline::EmbeddingPipeline;
use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_runtime::llama_model_loader::LlamaModelLoader;
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
    fn into_established_capability(self) -> ControlFlow<CapabilityStartupInterruption, TStarted>;
}

impl<TStarted, TError> IntoEstablishedCapability<TStarted>
    for Result<StartupOutcome<TStarted>, TError>
where
    AgentError: From<TError>,
{
    fn into_established_capability(self) -> ControlFlow<CapabilityStartupInterruption, TStarted> {
        match self {
            Ok(StartupOutcome::Ready(started)) => ControlFlow::Continue(started),
            Ok(StartupOutcome::Cancelled) => {
                ControlFlow::Break(CapabilityStartupInterruption::Cancelled)
            }
            Err(startup_error) => ControlFlow::Break(CapabilityStartupInterruption::Failed(
                Box::new(AgentError::from(startup_error)),
            )),
        }
    }
}

pub struct CapabilityStarter<'startup> {
    pub cancellation_token: &'startup CancellationToken,
    pub inference_runtime_context: &'startup InferenceRuntimeContext,
}

impl CapabilityStarter<'_> {
    pub async fn load_model(
        &self,
        llama_model_settings: LlamaModelSettings,
    ) -> ControlFlow<CapabilityStartupInterruption, LoadedLlamaModel> {
        LlamaModelLoader {
            inference_runtime_context: self.inference_runtime_context.clone(),
            llama_model_settings,
        }
        .load_unless_shutting_down(self.cancellation_token)
        .await
        .into_established_capability()
    }

    pub async fn prepare_decisions(
        &self,
        applicable_decisions: &ApplicableDecisions,
    ) -> ControlFlow<CapabilityStartupInterruption, PreparedDecisions> {
        match applicable_decisions {
            ApplicableDecisions::Disabled => ControlFlow::Continue(PreparedDecisions::Disabled),
            ApplicableDecisions::Enabled { pointer_head_path } => {
                let decision_prerequisites = DecisionPrerequisites {
                    inference_runtime_context: self.inference_runtime_context.clone(),
                    pointer_head_path: pointer_head_path.clone(),
                };
                let cancellation_token = self.cancellation_token.clone();

                spawn_blocking_resuming_panic(move || {
                    decision_prerequisites
                        .meet()
                        .map(|met_decision_prerequisites| {
                            if cancellation_token.is_cancelled() {
                                StartupOutcome::Cancelled
                            } else {
                                StartupOutcome::Ready(met_decision_prerequisites)
                            }
                        })
                })
                .await
                .into_established_capability()
                .map_continue(|met_decision_prerequisites| {
                    PreparedDecisions::Enabled(Box::new(met_decision_prerequisites))
                })
            }
        }
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
    ) -> ControlFlow<Box<ServingInterruption>, ServedModel> {
        let decisions = match self
            .establish_decisions(
                &loaded_llama_model,
                &model_serving_settings.model_runtime_parameters,
                decisions_plan,
                prepared_decisions,
            )
            .await
        {
            ControlFlow::Continue(decisions) => decisions,
            ControlFlow::Break(interruption) => {
                embeddings_plan.shut_down().await;
                text_generation_plan.shut_down().await;

                return ControlFlow::Break(Box::new(ServingInterruption {
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
            ControlFlow::Continue(embeddings) => embeddings,
            ControlFlow::Break(interruption) => {
                decisions.shut_down().await;
                text_generation_plan.shut_down().await;

                return ControlFlow::Break(Box::new(ServingInterruption {
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
            ControlFlow::Continue(text_generation) => text_generation,
            ControlFlow::Break(interruption) => {
                decisions.shut_down().await;
                embeddings.shut_down().await;

                return ControlFlow::Break(Box::new(ServingInterruption {
                    interruption,
                    loaded_llama_model,
                }));
            }
        };

        ControlFlow::Continue(ServedModel {
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
    ) -> ControlFlow<CapabilityStartupInterruption, ServedDecisions> {
        match (decisions_plan, prepared_decisions) {
            (CapabilityPlan::Keep(served_decisions), _) => ControlFlow::Continue(served_decisions),
            (CapabilityPlan::Start, PreparedDecisions::Disabled) => {
                ControlFlow::Continue(ServedDecisions::Disabled)
            }
            (CapabilityPlan::Start, PreparedDecisions::Enabled(met_decision_prerequisites)) => {
                DecisionPipeline {
                    inference_runtime_context: self.inference_runtime_context.clone(),
                    loaded_llama_model: loaded_llama_model.clone(),
                    met_decision_prerequisites: *met_decision_prerequisites,
                    model_runtime_parameters: model_runtime_parameters.clone(),
                }
                .spawn(self.cancellation_token)
                .await
                .into_established_capability()
                .map_continue(ServedDecisions::Enabled)
            }
        }
    }

    async fn establish_embeddings(
        &self,
        loaded_llama_model: &LoadedLlamaModel,
        ModelServingSettings {
            capabilities: InferenceCapabilities { embeddings, .. },
            model_runtime_parameters,
            ..
        }: &ModelServingSettings,
        embeddings_plan: CapabilityPlan<ServedEmbeddings>,
    ) -> ControlFlow<CapabilityStartupInterruption, ServedEmbeddings> {
        match embeddings_plan {
            CapabilityPlan::Keep(served_embeddings) => ControlFlow::Continue(served_embeddings),
            CapabilityPlan::Start => EmbeddingPipeline {
                inference_runtime_context: self.inference_runtime_context.clone(),
                loaded_llama_model: loaded_llama_model.clone(),
                model_runtime_parameters: model_runtime_parameters.clone(),
                pooling_type: embeddings.clone(),
            }
            .spawn(self.cancellation_token)
            .await
            .into_established_capability(),
        }
    }

    async fn establish_text_generation(
        &self,
        loaded_llama_model: &LoadedLlamaModel,
        ModelServingSettings {
            capabilities: InferenceCapabilities {
                text_generation, ..
            },
            model_runtime_parameters,
            ..
        }: &ModelServingSettings,
        text_generation_plan: CapabilityPlan<ServedTextGeneration>,
    ) -> ControlFlow<CapabilityStartupInterruption, ServedTextGeneration> {
        match text_generation_plan {
            CapabilityPlan::Keep(served_text_generation) => {
                ControlFlow::Continue(served_text_generation)
            }
            CapabilityPlan::Start => TextGenerationPipeline {
                inference_runtime_context: self.inference_runtime_context.clone(),
                loaded_llama_model: loaded_llama_model.clone(),
                model_runtime_parameters: model_runtime_parameters.clone(),
                text_generation_settings: text_generation.clone(),
            }
            .spawn(self.cancellation_token)
            .await
            .into_established_capability(),
        }
    }
}
