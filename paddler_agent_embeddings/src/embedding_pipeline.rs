use std::sync::Arc;

use llama_cpp_bindings::llama_batch::LlamaBatch;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::mpsc::unbounded_channel;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use paddler_agent_runtime::continue_startup_unless_shutting_down::continue_startup_unless_shutting_down;
use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_agent_runtime::run_scheduler_unless_startup_abandoned::run_scheduler_unless_startup_abandoned;
use paddler_agent_runtime::scheduler_handle::SchedulerHandle;
use paddler_agent_runtime::scheduler_inbox::SchedulerInbox;
use paddler_agent_runtime::scheduler_ready::SchedulerReady;
use paddler_agent_runtime::scheduler_request_preparer::SchedulerRequestPreparer;
use paddler_agent_runtime::start_scheduler_thread::start_scheduler_thread;
use paddler_agent_runtime::startup_outcome::StartupOutcome;
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
    pub loaded_llama_model: LoadedLlamaModel,
    pub model_runtime_parameters: ModelRuntimeParameters,
    pub pooling_type: PoolingType,
}

impl EmbeddingPipeline {
    pub async fn spawn(
        self,
        cancellation_token: &CancellationToken,
    ) -> Result<StartupOutcome<SchedulerHandle<EmbeddingBatchPreparer>>, EmbeddingError> {
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
            loaded_llama_model,
            model_runtime_parameters,
            pooling_type,
        } = self;
        let desired_slots_total = inference_runtime_context
            .slot_aggregated_status
            .desired_slots_total;
        let n_batch = model_runtime_parameters.n_batch.tokens_usize();

        continue_startup_unless_shutting_down(agent_shutdown, || {
            let llama_context = EmbeddingLlamaContextSettings {
                llama_context_settings: loaded_llama_model.llama_context_settings(
                    &model_runtime_parameters,
                    u32::from(desired_slots_total),
                ),
                pooling_type: pooling_type.clone(),
            }
            .create_llama_context(&loaded_llama_model, &inference_runtime_context)?;
            let batch = LlamaBatch::new(
                model_runtime_parameters.n_batch.capacity(),
                i32::from(desired_slots_total),
            );

            continue_startup_unless_shutting_down(agent_shutdown, || {
                loaded_llama_model
                    .warm_up_and_serve(
                        llama_context,
                        batch,
                        desired_slots_total,
                        |llama_context, batch| {
                            let (scheduler_message_tx, scheduler_message_rx) = unbounded_channel();

                            run_scheduler_unless_startup_abandoned(
                                scheduler_startup_tx,
                                Ok(SchedulerReady {
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
                                        active_requests: Vec::new(),
                                        agent_shutdown: agent_shutdown.clone(),
                                        batch,
                                        llama_context,
                                        scheduler_context: EmbeddingSchedulerContext {
                                            agent_name: inference_runtime_context.agent_name,
                                            desired_slots_total,
                                            n_batch,
                                            pooling_type,
                                        },
                                        scheduler_inbox: SchedulerInbox::new(scheduler_message_rx),
                                    }
                                    .run();
                                },
                            );
                        },
                    )
                    .map_err(EmbeddingError::from)
            })
        })
    }
}
