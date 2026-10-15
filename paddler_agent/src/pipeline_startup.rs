use std::future::Future;
use std::future::pending;
use std::mem;
use std::pin::Pin;

use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use paddler_agent_decision::decision_pipeline::DecisionPipeline;
use paddler_agent_decision::general_architecture_metadata::GeneralArchitectureMetadata;
use paddler_agent_embeddings::embedding_pipeline::EmbeddingPipeline;
use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_runtime::prepares_scheduler_command::PreparesSchedulerCommand;
use paddler_agent_runtime::scheduler_handle::SchedulerHandle;
use paddler_agent_runtime::scheduler_spawn_outcome::SchedulerSpawnOutcome;
use paddler_agent_text_generation::llama_cpp_media_marker::LlamaCppMediaMarker;
use paddler_agent_text_generation::text_generation_pipeline::TextGenerationPipeline;
use paddler_messaging::agent_issue_params::model_path::ModelPath;

use crate::agent_applicable_state::AgentApplicableState;
use crate::agent_error::AgentError;
use crate::finished_pipeline_startup::FinishedPipelineStartup;
use crate::retry_delay::RetryDelay;
use crate::running_pipeline::RunningPipeline;
use crate::started_pipeline::StartedPipeline;

fn started<TPreparation>(
    model_path: ModelPath,
    scheduler_spawn_outcome: SchedulerSpawnOutcome<TPreparation>,
    running_pipeline: fn(SchedulerHandle<TPreparation>) -> RunningPipeline,
) -> StartedPipeline
where
    TPreparation: PreparesSchedulerCommand,
{
    match scheduler_spawn_outcome {
        SchedulerSpawnOutcome::Ready(scheduler_handle) => StartedPipeline::Running {
            model_path,
            running_pipeline: running_pipeline(scheduler_handle),
        },
        SchedulerSpawnOutcome::Cancelled => StartedPipeline::Cancelled,
    }
}

async fn start_pipeline(
    cancellation_token: &CancellationToken,
    agent_applicable_state: AgentApplicableState,
    inference_runtime_context: &InferenceRuntimeContext,
) -> Result<StartedPipeline, AgentError> {
    Ok(match agent_applicable_state {
        AgentApplicableState::Decision {
            model_path,
            model_runtime_parameters,
            pointer_head_path,
        } => started(
            ModelPath::from(model_path.as_path()),
            DecisionPipeline {
                architecture: GeneralArchitectureMetadata,
                inference_runtime_context: inference_runtime_context.clone(),
                model_path,
                model_runtime_parameters,
                pointer_head_path,
            }
            .spawn(cancellation_token)
            .await?,
            RunningPipeline::Decision,
        ),
        AgentApplicableState::Embeddings {
            model_path,
            model_runtime_parameters,
            pooling_type,
        } => started(
            ModelPath::from(model_path.as_path()),
            EmbeddingPipeline {
                inference_runtime_context: inference_runtime_context.clone(),
                model_path,
                model_runtime_parameters,
                pooling_type,
            }
            .spawn(cancellation_token)
            .await?,
            RunningPipeline::Embeddings,
        ),
        AgentApplicableState::TextGeneration {
            model_path,
            model_runtime_parameters,
            text_generation_settings,
        } => started(
            ModelPath::from(model_path.as_path()),
            TextGenerationPipeline {
                inference_runtime_context: inference_runtime_context.clone(),
                media_marker: LlamaCppMediaMarker,
                model_path,
                model_runtime_parameters,
                text_generation_settings,
            }
            .spawn(cancellation_token)
            .await?,
            RunningPipeline::TextGeneration,
        ),
        AgentApplicableState::NotConfigured
        | AgentApplicableState::TextGenerationWithoutModel { .. } => {
            StartedPipeline::NothingToServe
        }
    })
}

pub enum PipelineStartup {
    Idle,
    Starting {
        cancellation_token: CancellationToken,
        finished: Pin<Box<dyn Future<Output = FinishedPipelineStartup> + Send>>,
    },
}

impl PipelineStartup {
    #[must_use]
    pub fn start(
        agent_applicable_state: AgentApplicableState,
        agent_shutdown: &CancellationToken,
        inference_runtime_context: InferenceRuntimeContext,
        retry_delay: RetryDelay,
    ) -> Self {
        let cancellation_token = agent_shutdown.child_token();
        let startup_cancellation_token = cancellation_token.clone();

        Self::Starting {
            cancellation_token,
            finished: Box::pin(async move {
                retry_delay.elapse(&startup_cancellation_token).await;

                FinishedPipelineStartup {
                    outcome: start_pipeline(
                        &startup_cancellation_token,
                        agent_applicable_state.clone(),
                        &inference_runtime_context,
                    )
                    .await,
                    agent_applicable_state,
                }
            }),
        }
    }

    pub fn abandon(&mut self, task_tracker: &TaskTracker) {
        if let Self::Starting {
            cancellation_token,
            finished,
        } = mem::replace(self, Self::Idle)
        {
            cancellation_token.cancel();
            task_tracker.spawn(finished);
        }
    }

    pub async fn finish(&mut self) -> FinishedPipelineStartup {
        match self {
            Self::Idle => pending().await,
            Self::Starting { finished, .. } => {
                let finished_pipeline_startup = finished.as_mut().await;

                *self = Self::Idle;

                finished_pipeline_startup
            }
        }
    }
}
