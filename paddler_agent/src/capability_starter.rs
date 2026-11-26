use std::ops::ControlFlow;
use std::sync::Arc;

use llama_cpp_bindings_types::embeddings_capability::EmbeddingsCapability;
use llama_cpp_bindings_types::text_generation_capability::TextGenerationCapability;
use llama_cpp_model_capabilities::detected_decisions::DetectedDecisions;
use tokio_util::sync::CancellationToken;

use paddler_agent_decision::decision_pipeline::DecisionPipeline;
use paddler_agent_decision::decision_slots::DecisionSlots;
use paddler_agent_embeddings::embedding_pipeline::EmbeddingPipeline;
use paddler_agent_model_capabilities::established_model_capabilities::EstablishedModelCapabilities;
use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_runtime::llama_model_loader::LlamaModelLoader;
use paddler_agent_runtime::llama_model_settings::LlamaModelSettings;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_agent_runtime::shuts_down::ShutsDown as _;
use paddler_agent_runtime::startup_outcome::StartupOutcome;
use paddler_agent_text_generation::text_generation_pipeline::TextGenerationPipeline;
use paddler_spawn_resuming_panic::spawn_blocking_resuming_panic::spawn_blocking_resuming_panic;

use crate::agent_error::AgentError;
use crate::capability_startup_interruption::CapabilityStartupInterruption;
use crate::loaded_model_serving_plan::LoadedModelServingPlan;
use crate::model_serving_settings::ModelServingSettings;
use crate::served_capabilities::ServedCapabilities;
use crate::served_decisions::ServedDecisions;
use crate::served_embeddings::ServedEmbeddings;
use crate::served_model::ServedModel;
use crate::served_text_generation::ServedTextGeneration;
use crate::serving_interruption::ServingInterruption;

fn startup_failure<TError>(startup_error: TError) -> CapabilityStartupInterruption
where
    AgentError: From<TError>,
{
    CapabilityStartupInterruption::Failed(Box::new(AgentError::from(startup_error)))
}

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

    pub async fn serve(
        &self,
        loaded_model_serving_plan: LoadedModelServingPlan,
    ) -> ControlFlow<Box<ServingInterruption>, ServedModel> {
        match loaded_model_serving_plan {
            LoadedModelServingPlan::RestartingTextGeneration {
                decisions,
                embeddings,
                loaded_llama_model,
                model_capabilities,
                model_serving_settings,
            } => {
                let text_generation_start = self
                    .complete_with_text_generation(
                        &loaded_llama_model,
                        &model_serving_settings,
                        &model_capabilities,
                        decisions,
                        embeddings,
                    )
                    .await;

                Self::served_model_of(
                    text_generation_start,
                    loaded_llama_model,
                    model_capabilities,
                    model_serving_settings,
                )
            }
            LoadedModelServingPlan::StartingEveryCapability {
                loaded_llama_model,
                model_serving_settings,
            } => {
                let model_capabilities = match self
                    .establish_capabilities(&loaded_llama_model, &model_serving_settings)
                    .await
                {
                    ControlFlow::Continue(model_capabilities) => model_capabilities,
                    ControlFlow::Break(interruption) => {
                        return ControlFlow::Break(Box::new(ServingInterruption {
                            interruption,
                            loaded_llama_model,
                        }));
                    }
                };
                let capabilities_start = self
                    .start_every_capability(
                        &loaded_llama_model,
                        &model_serving_settings,
                        &model_capabilities,
                    )
                    .await;

                Self::served_model_of(
                    capabilities_start,
                    loaded_llama_model,
                    model_capabilities,
                    model_serving_settings,
                )
            }
        }
    }

    async fn complete_with_text_generation(
        &self,
        loaded_llama_model: &LoadedLlamaModel,
        model_serving_settings: &ModelServingSettings,
        model_capabilities: &Arc<EstablishedModelCapabilities>,
        decisions: ServedDecisions,
        embeddings: ServedEmbeddings,
    ) -> ControlFlow<CapabilityStartupInterruption, ServedCapabilities> {
        match self
            .start_text_generation(
                loaded_llama_model,
                model_serving_settings,
                model_capabilities,
            )
            .await
        {
            ControlFlow::Continue(text_generation) => ControlFlow::Continue(ServedCapabilities {
                decisions,
                embeddings,
                text_generation,
            }),
            ControlFlow::Break(interruption) => {
                decisions.shut_down().await;
                embeddings.shut_down().await;

                ControlFlow::Break(interruption)
            }
        }
    }

    async fn establish_capabilities(
        &self,
        loaded_llama_model: &LoadedLlamaModel,
        ModelServingSettings {
            model_capabilities, ..
        }: &ModelServingSettings,
    ) -> ControlFlow<CapabilityStartupInterruption, Arc<EstablishedModelCapabilities>> {
        let loaded_llama_model = loaded_llama_model.clone();
        let inference_runtime_context = self.inference_runtime_context.clone();
        let model_capabilities_settings = model_capabilities.clone();

        match spawn_blocking_resuming_panic(move || {
            EstablishedModelCapabilities::establish(
                &loaded_llama_model,
                &inference_runtime_context,
                model_capabilities_settings,
            )
        })
        .await
        {
            Ok(model_capabilities) => ControlFlow::Continue(Arc::new(model_capabilities)),
            Err(establishment_error) => ControlFlow::Break(startup_failure(establishment_error)),
        }
    }

    fn served_model_of(
        capabilities_start: ControlFlow<CapabilityStartupInterruption, ServedCapabilities>,
        loaded_llama_model: LoadedLlamaModel,
        model_capabilities: Arc<EstablishedModelCapabilities>,
        model_serving_settings: ModelServingSettings,
    ) -> ControlFlow<Box<ServingInterruption>, ServedModel> {
        match capabilities_start {
            ControlFlow::Continue(capabilities) => ControlFlow::Continue(ServedModel {
                capabilities,
                loaded_llama_model,
                model_capabilities,
                model_serving_settings,
            }),
            ControlFlow::Break(interruption) => {
                drop(model_capabilities);

                ControlFlow::Break(Box::new(ServingInterruption {
                    interruption,
                    loaded_llama_model,
                }))
            }
        }
    }

    async fn start_decisions(
        &self,
        loaded_llama_model: &LoadedLlamaModel,
        ModelServingSettings {
            model_runtime_parameters,
            ..
        }: &ModelServingSettings,
        model_capabilities: &Arc<EstablishedModelCapabilities>,
    ) -> ControlFlow<CapabilityStartupInterruption, ServedDecisions> {
        let DetectedDecisions::Supported(decision_model) = &model_capabilities.detected.decisions
        else {
            return ControlFlow::Continue(ServedDecisions::NotServed);
        };
        let decision_slots =
            match DecisionSlots::allot(self.inference_runtime_context, decision_model) {
                Ok(decision_slots) => decision_slots,
                Err(slots_insufficient) => {
                    return ControlFlow::Break(startup_failure(slots_insufficient));
                }
            };

        DecisionPipeline {
            decision_model: decision_model.clone(),
            decision_slots,
            inference_runtime_context: self.inference_runtime_context.clone(),
            loaded_llama_model: loaded_llama_model.clone(),
            model_capabilities: model_capabilities.clone(),
            model_runtime_parameters: model_runtime_parameters.clone(),
        }
        .spawn(self.cancellation_token)
        .await
        .into_established_capability()
        .map_continue(ServedDecisions::Served)
    }

    async fn start_embeddings(
        &self,
        loaded_llama_model: &LoadedLlamaModel,
        ModelServingSettings {
            model_capabilities: model_capabilities_settings,
            model_runtime_parameters,
            ..
        }: &ModelServingSettings,
        model_capabilities: &Arc<EstablishedModelCapabilities>,
    ) -> ControlFlow<CapabilityStartupInterruption, ServedEmbeddings> {
        let EmbeddingsCapability::Supported = model_capabilities.detected.embeddings else {
            return ControlFlow::Continue(ServedEmbeddings::NotServed);
        };

        EmbeddingPipeline {
            inference_runtime_context: self.inference_runtime_context.clone(),
            loaded_llama_model: loaded_llama_model.clone(),
            model_runtime_parameters: model_runtime_parameters.clone(),
            pooling_type: model_capabilities_settings.embedding_pooling.clone(),
        }
        .spawn(self.cancellation_token)
        .await
        .into_established_capability()
        .map_continue(ServedEmbeddings::Served)
    }

    async fn start_every_capability(
        &self,
        loaded_llama_model: &LoadedLlamaModel,
        model_serving_settings: &ModelServingSettings,
        model_capabilities: &Arc<EstablishedModelCapabilities>,
    ) -> ControlFlow<CapabilityStartupInterruption, ServedCapabilities> {
        let decisions = self
            .start_decisions(
                loaded_llama_model,
                model_serving_settings,
                model_capabilities,
            )
            .await?;
        let embeddings = match self
            .start_embeddings(
                loaded_llama_model,
                model_serving_settings,
                model_capabilities,
            )
            .await
        {
            ControlFlow::Continue(embeddings) => embeddings,
            ControlFlow::Break(interruption) => {
                decisions.shut_down().await;

                return ControlFlow::Break(interruption);
            }
        };

        self.complete_with_text_generation(
            loaded_llama_model,
            model_serving_settings,
            model_capabilities,
            decisions,
            embeddings,
        )
        .await
    }

    async fn start_text_generation(
        &self,
        loaded_llama_model: &LoadedLlamaModel,
        ModelServingSettings {
            model_runtime_parameters,
            text_generation,
            ..
        }: &ModelServingSettings,
        model_capabilities: &Arc<EstablishedModelCapabilities>,
    ) -> ControlFlow<CapabilityStartupInterruption, ServedTextGeneration> {
        let TextGenerationCapability::Supported = model_capabilities.detected.text_generation
        else {
            return ControlFlow::Continue(ServedTextGeneration::NotServed);
        };

        TextGenerationPipeline {
            inference_runtime_context: self.inference_runtime_context.clone(),
            loaded_llama_model: loaded_llama_model.clone(),
            model_capabilities: model_capabilities.clone(),
            model_runtime_parameters: model_runtime_parameters.clone(),
            text_generation_settings: text_generation.clone(),
        }
        .spawn(self.cancellation_token)
        .await
        .into_established_capability()
        .map_continue(ServedTextGeneration::Served)
    }
}
