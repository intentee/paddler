use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::channel;
use std::thread;

use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::model::params::LlamaModelParams;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_agent_runtime::await_scheduler_startup::await_scheduler_startup;
use paddler_agent_runtime::continue_startup_unless_shutting_down::continue_startup_unless_shutting_down;
use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_runtime::inference_thread_count::inference_thread_count;
use paddler_agent_runtime::llama_context_settings::LlamaContextSettings;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_agent_runtime::run_scheduler_unless_startup_abandoned::run_scheduler_unless_startup_abandoned;
use paddler_agent_runtime::scheduler_request_preparer::SchedulerRequestPreparer;
use paddler_agent_runtime::scheduler_spawn_outcome::SchedulerSpawnOutcome;
use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_messaging::agent_issue_params::model_path::ModelPath;

use crate::agent_pooling_type::AgentPoolingType;
use crate::converts_to_llama_pooling_type::ConvertsToLlamaPoolingType as _;
use crate::embedding_batch_preparer::EmbeddingBatchPreparer;
use crate::embedding_error::EmbeddingError;
use crate::embedding_scheduler::EmbeddingScheduler;
use crate::embedding_scheduler_context::EmbeddingSchedulerContext;

pub struct EmbeddingPipeline {
    pub embedding_parameters: EmbeddingParameters,
    pub inference_runtime_context: InferenceRuntimeContext,
    pub model_path: PathBuf,
    pub model_runtime_parameters: ModelRuntimeParameters,
}

impl EmbeddingPipeline {
    pub async fn spawn(
        self,
        cancellation_token: &CancellationToken,
    ) -> Result<SchedulerSpawnOutcome<EmbeddingBatchPreparer, EmbeddingError>, EmbeddingError> {
        let (scheduler_ready_tx, scheduler_ready_rx) = oneshot::channel();
        let agent_shutdown = cancellation_token.clone();

        let scheduler_thread_handle =
            thread::spawn(move || self.run_scheduler(&agent_shutdown, scheduler_ready_tx));

        await_scheduler_startup(
            cancellation_token,
            scheduler_ready_rx,
            scheduler_thread_handle,
        )
        .await
    }

    fn run_scheduler(
        self,
        agent_shutdown: &CancellationToken,
        scheduler_ready_tx: oneshot::Sender<Arc<SchedulerRequestPreparer<EmbeddingBatchPreparer>>>,
    ) -> Result<(), EmbeddingError> {
        let Self {
            embedding_parameters,
            inference_runtime_context,
            model_path,
            model_runtime_parameters,
        } = self;
        let desired_slots_total = inference_runtime_context
            .slot_aggregated_status
            .desired_slots_total;
        let n_batch = model_runtime_parameters.n_batch.tokens_usize();

        continue_startup_unless_shutting_down(agent_shutdown, || {
            let thread_count = inference_thread_count()?;
            let loaded_llama_model = LoadedLlamaModel::load(
                &inference_runtime_context,
                &model_path,
                &LlamaModelParams::default()
                    .with_n_gpu_layers(model_runtime_parameters.n_gpu_layers),
            )?;

            continue_startup_unless_shutting_down(agent_shutdown, || {
                inference_runtime_context
                    .slot_aggregated_status
                    .set_model_path(Some(ModelPath::from(model_path.as_path()).model_path));

                let mut llama_context = loaded_llama_model.create_llama_context(
                    &inference_runtime_context,
                    LlamaContextSettings {
                        model_runtime_parameters: &model_runtime_parameters,
                        n_seq_max: u32::from(desired_slots_total),
                        thread_count,
                    }
                    .into_llama_context_params()
                    .with_embeddings(true)
                    .with_n_ubatch(model_runtime_parameters.n_batch.tokens().get())
                    .with_pooling_type(
                        AgentPoolingType(embedding_parameters.pooling_type.clone())
                            .to_llama_pooling_type(),
                    ),
                )?;
                let mut batch = LlamaBatch::new(n_batch, i32::from(desired_slots_total))
                    .map_err(AgentRuntimeError::BatchAllocationFailed)?;

                continue_startup_unless_shutting_down(agent_shutdown, || {
                    loaded_llama_model.warm_up_llama_context(
                        &mut llama_context,
                        &mut batch,
                        desired_slots_total,
                    );

                    let (scheduler_message_tx, scheduler_message_rx) = channel();

                    run_scheduler_unless_startup_abandoned(
                        scheduler_ready_tx,
                        Arc::new(SchedulerRequestPreparer {
                            agent_name: inference_runtime_context.agent_name.clone(),
                            preparation: EmbeddingBatchPreparer {
                                loaded_llama_model: loaded_llama_model.clone(),
                                n_batch,
                            },
                            preparation_tasks: TaskTracker::new(),
                            scheduler_message_tx,
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
                                    pooling_type: embedding_parameters.pooling_type,
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
