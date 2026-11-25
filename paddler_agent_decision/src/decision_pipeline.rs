use std::collections::VecDeque;
use std::num::NonZeroU16;
use std::num::NonZeroU32;
use std::sync::Arc;

use llama_cpp_decision::decision_model::DecisionModel;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::mpsc::unbounded_channel;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use paddler_agent_model_capabilities::established_model_capabilities::EstablishedModelCapabilities;
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
use paddler_inference_parameters::desired_model_runtime_parameters::DesiredModelRuntimeParameters;

use crate::decision_capacity_ledger::DecisionCapacityLedger;
use crate::decision_error::DecisionError;
use crate::decision_llama_context_settings::DecisionLlamaContextSettings;
use crate::decision_request_preparer::DecisionRequestPreparer;
use crate::decision_scheduler::DecisionScheduler;
use crate::decision_scheduler_context::DecisionSchedulerContext;

pub struct DecisionPipeline {
    pub decision_model: DecisionModel,
    pub decision_slots: NonZeroU16,
    pub inference_runtime_context: InferenceRuntimeContext,
    pub loaded_llama_model: LoadedLlamaModel,
    pub model_capabilities: Arc<EstablishedModelCapabilities>,
    pub model_runtime_parameters: DesiredModelRuntimeParameters,
}

impl DecisionPipeline {
    /// # Errors
    ///
    /// Returns [`DecisionError::AgentRuntime`] when the decision context cannot be created or warmed up.
    pub async fn spawn(
        self,
        cancellation_token: &CancellationToken,
    ) -> Result<StartupOutcome<SchedulerHandle<DecisionRequestPreparer>>, DecisionError> {
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
            Result<SchedulerReady<DecisionRequestPreparer>, DecisionError>,
        >,
    ) -> Result<(), DecisionError> {
        continue_startup_unless_shutting_down(agent_shutdown, || {
            let decision_slots = self.decision_slots;
            let llama_context = DecisionLlamaContextSettings {
                decision_model: &self.decision_model,
                llama_context_settings: self.loaded_llama_model.llama_context_settings(
                    &self.model_runtime_parameters,
                    u32::from(decision_slots.get()),
                ),
                model_capabilities: &self.model_capabilities,
            }
            .create_llama_context(&self.loaded_llama_model, &self.inference_runtime_context)?;

            continue_startup_unless_shutting_down(agent_shutdown, || {
                self.loaded_llama_model
                    .warm_up_and_serve(llama_context, |llama_context, batch| {
                        let cells_per_slot =
                            llama_context.n_ctx() / NonZeroU32::from(decision_slots);
                        let (scheduler_message_tx, scheduler_message_rx) = unbounded_channel();

                        run_scheduler_unless_startup_abandoned(
                            scheduler_startup_tx,
                            Ok(SchedulerReady {
                                request_preparer: Arc::new(SchedulerRequestPreparer {
                                    agent_name: self.inference_runtime_context.agent_name.clone(),
                                    preparation: DecisionRequestPreparer {
                                        batch_tokens: batch.limits().batch as usize,
                                        cells_per_slot,
                                        decision_model: self.decision_model.clone(),
                                        model_capabilities: self.model_capabilities.clone(),
                                    },
                                    preparation_tasks: TaskTracker::new(),
                                    scheduler_message_tx,
                                }),
                            }),
                            || {
                                DecisionScheduler {
                                    active_requests: VecDeque::new(),
                                    agent_shutdown: agent_shutdown.clone(),
                                    batch,
                                    capacity_ledger: DecisionCapacityLedger::new(
                                        decision_slots.get(),
                                    ),
                                    llama_context,
                                    pending_requests: VecDeque::new(),
                                    scheduler_context: DecisionSchedulerContext {
                                        agent_name: self.inference_runtime_context.agent_name,
                                        model_capabilities: self.model_capabilities,
                                    },
                                    scheduler_inbox: SchedulerInbox::new(scheduler_message_rx),
                                }
                                .run();
                            },
                        );
                    })
                    .map_err(DecisionError::from)
            })
        })
    }
}
