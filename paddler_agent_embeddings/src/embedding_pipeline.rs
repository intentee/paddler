use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::channel;

use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::model::params::LlamaModelParams;
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use paddler_agent_runtime::continue_startup_unless_shutting_down::continue_startup_unless_shutting_down;
use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_runtime::inference_thread_count::inference_thread_count;
use paddler_agent_runtime::llama_context_settings::LlamaContextSettings;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_agent_runtime::run_scheduler_unless_startup_abandoned::run_scheduler_unless_startup_abandoned;
use paddler_agent_runtime::scheduler_ready::SchedulerReady;
use paddler_agent_runtime::scheduler_request_preparer::SchedulerRequestPreparer;
use paddler_agent_runtime::scheduler_spawn_outcome::SchedulerSpawnOutcome;
use paddler_agent_runtime::start_scheduler_thread::start_scheduler_thread;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_inference_parameters::pooling_type::PoolingType;

use crate::embedding_batch_preparer::EmbeddingBatchPreparer;
use crate::embedding_error::EmbeddingError;
use crate::embedding_llama_context_settings::EmbeddingLlamaContextSettings;
use crate::embedding_scheduler::EmbeddingScheduler;
use crate::embedding_scheduler_context::EmbeddingSchedulerContext;
use crate::sequence_context_limit::SequenceContextLimit;

pub struct EmbeddingPipeline {
    pub inference_runtime_context: InferenceRuntimeContext,
    pub model_path: PathBuf,
    pub model_runtime_parameters: ModelRuntimeParameters,
    pub pooling_type: PoolingType,
}

impl EmbeddingPipeline {
    pub async fn spawn(
        self,
        cancellation_token: &CancellationToken,
    ) -> Result<SchedulerSpawnOutcome<EmbeddingBatchPreparer>, EmbeddingError> {
        start_scheduler_thread(
            cancellation_token,
            move |agent_shutdown, scheduler_startup_tx| {
                self.run_scheduler(agent_shutdown, scheduler_startup_tx)
            },
        )
        .await
    }

    fn run_scheduler(
        self,
        agent_shutdown: &CancellationToken,
        scheduler_startup_tx: &UnboundedSender<
            Result<SchedulerReady<EmbeddingBatchPreparer>, EmbeddingError>,
        >,
    ) -> Result<(), EmbeddingError> {
        let Self {
            inference_runtime_context,
            model_path,
            model_runtime_parameters,
            pooling_type,
        } = self;
        let desired_slots_total = inference_runtime_context
            .slot_aggregated_status
            .desired_slots_total;
        let n_batch = model_runtime_parameters.n_batch.tokens_usize();

        continue_startup_unless_shutting_down(agent_shutdown, || {
            let thread_count =
                inference_thread_count(inference_runtime_context.available_parallelism.as_ref())?;
            let loaded_llama_model = LoadedLlamaModel::load(
                &inference_runtime_context,
                &model_path,
                &LlamaModelParams::default()
                    .with_n_gpu_layers(model_runtime_parameters.n_gpu_layers),
            )?;

            continue_startup_unless_shutting_down(agent_shutdown, || {
                let mut llama_context = EmbeddingLlamaContextSettings {
                    llama_context_settings: LlamaContextSettings {
                        model_runtime_parameters: &model_runtime_parameters,
                        n_seq_max: u32::from(desired_slots_total),
                        thread_count,
                    },
                    pooling_type: pooling_type.clone(),
                }
                .create_llama_context(&loaded_llama_model, &inference_runtime_context)?;
                let mut batch = LlamaBatch::new(
                    model_runtime_parameters.n_batch.capacity(),
                    i32::from(desired_slots_total),
                );

                continue_startup_unless_shutting_down(agent_shutdown, || {
                    loaded_llama_model.warm_up_llama_context(
                        &mut llama_context,
                        &mut batch,
                        desired_slots_total,
                    );

                    let (scheduler_message_tx, scheduler_message_rx) = channel();

                    run_scheduler_unless_startup_abandoned(
                        scheduler_startup_tx,
                        Ok(SchedulerReady {
                            model_metadata: loaded_llama_model.model_metadata.clone(),
                            request_preparer: Arc::new(SchedulerRequestPreparer {
                                agent_name: inference_runtime_context.agent_name.clone(),
                                preparation: EmbeddingBatchPreparer {
                                    loaded_llama_model: loaded_llama_model.clone(),
                                    n_batch,
                                    sequence_context_limit: SequenceContextLimit::of(
                                        &llama_context,
                                    ),
                                },
                                preparation_tasks: TaskTracker::new(),
                                scheduler_message_tx,
                            }),
                        }),
                        || {
                            EmbeddingScheduler {
                                agent_shutdown: agent_shutdown.clone(),
                                batch,
                                scheduler_message_rx,
                                llama_context,
                                scheduler_context: EmbeddingSchedulerContext {
                                    agent_name: inference_runtime_context.agent_name,
                                    desired_slots_total,
                                    n_batch,
                                    pooling_type,
                                },
                            }
                            .run();
                        },
                    );

                    Ok(())
                })
            })
        })
    }
}
