use std::collections::VecDeque;
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
use paddler_agent_status::agent_issue_fix::AgentIssueFix;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::agent_issue_params::pointer_head_incompatibility::PointerHeadIncompatibility;
use paddler_messaging::agent_issue_params::pointer_head_incompatible_with_model_params::PointerHeadIncompatibleWithModelParams;

use crate::decision_capacity_ledger::DecisionCapacityLedger;
use crate::decision_delimiter_tokens::DecisionDelimiterTokens;
use crate::decision_error::DecisionError;
use crate::decision_llama_context_settings::DecisionLlamaContextSettings;
use crate::decision_request_preparer::DecisionRequestPreparer;
use crate::decision_scheduler::DecisionScheduler;
use crate::decision_scheduler_context::DecisionSchedulerContext;
use crate::decision_text_tokenizer::DecisionTextTokenizer;
use crate::loaded_pointer_head::LoadedPointerHead;

pub struct DecisionPipeline {
    pub inference_runtime_context: InferenceRuntimeContext,
    pub loaded_llama_model: LoadedLlamaModel,
    pub loaded_pointer_head: LoadedPointerHead,
    pub model_runtime_parameters: ModelRuntimeParameters,
}

impl DecisionPipeline {
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

    fn register_compatibility(&self) {
        self.inference_runtime_context
            .slot_aggregated_status
            .register_fix(&AgentIssueFix::PointerHeadIsCompatibleWithModel(
                ModelPath::from(self.loaded_pointer_head.pointer_head_path.as_path()),
            ));
    }

    fn register_incompatibility(
        &self,
        incompatibility: PointerHeadIncompatibility,
    ) -> DecisionError {
        self.inference_runtime_context
            .slot_aggregated_status
            .register_pipeline_issue(AgentIssue::PointerHeadIncompatibleWithModel(
                PointerHeadIncompatibleWithModelParams {
                    incompatibility: incompatibility.clone(),
                    pointer_head_path: ModelPath::from(
                        self.loaded_pointer_head.pointer_head_path.as_path(),
                    ),
                },
            ));

        DecisionError::PointerHeadIncompatibleWithModel { incompatibility }
    }

    fn resolve_delimiter_tokens(&self) -> Result<DecisionDelimiterTokens, DecisionError> {
        let model_hidden_size = self.loaded_llama_model.model.n_embd_out() as usize;
        let pointer_head = &self.loaded_pointer_head.pointer_head;

        if pointer_head.hidden_size() != model_hidden_size {
            return Err(self.register_incompatibility(
                PointerHeadIncompatibility::HiddenSizeMismatch {
                    model_hidden_size,
                    pointer_head_hidden_size: pointer_head.hidden_size(),
                },
            ));
        }

        match DecisionDelimiterTokens::resolve(
            &self.loaded_llama_model.model.llama_model,
            &pointer_head.delimiters,
        ) {
            Err(DecisionError::PointerHeadIncompatibleWithModel { incompatibility }) => {
                Err(self.register_incompatibility(incompatibility))
            }
            resolution => resolution.inspect(|_delimiter_tokens| self.register_compatibility()),
        }
    }

    fn run_scheduler(
        self,
        agent_shutdown: &CancellationToken,
        scheduler_startup_tx: &UnboundedSender<
            Result<SchedulerReady<DecisionRequestPreparer>, DecisionError>,
        >,
    ) -> Result<(), DecisionError> {
        continue_startup_unless_shutting_down(agent_shutdown, || {
            let delimiter_tokens = self.resolve_delimiter_tokens()?;
            let desired_slots_total = self
                .inference_runtime_context
                .slot_aggregated_status
                .desired_slots_total;
            let n_batch = self.model_runtime_parameters.n_batch.tokens_usize();
            let llama_context = DecisionLlamaContextSettings {
                llama_context_settings: self.loaded_llama_model.llama_context_settings(
                    &self.model_runtime_parameters,
                    u32::from(desired_slots_total),
                ),
            }
            .create_llama_context(&self.loaded_llama_model, &self.inference_runtime_context)?;
            let batch = LlamaBatch::new(self.model_runtime_parameters.n_batch.capacity(), 1);

            continue_startup_unless_shutting_down(agent_shutdown, || {
                self.loaded_llama_model
                    .warm_up_and_serve(
                        llama_context,
                        batch,
                        desired_slots_total,
                        |llama_context, batch| {
                            let cells_per_slot =
                                llama_context.n_ctx() / u32::from(desired_slots_total);
                            let (scheduler_message_tx, scheduler_message_rx) = unbounded_channel();

                            run_scheduler_unless_startup_abandoned(
                                scheduler_startup_tx,
                                Ok(SchedulerReady {
                                    request_preparer: Arc::new(SchedulerRequestPreparer {
                                        agent_name: self
                                            .inference_runtime_context
                                            .agent_name
                                            .clone(),
                                        preparation: DecisionRequestPreparer {
                                            cells_per_slot,
                                            delimiter_tokens,
                                            text_tokenizer: DecisionTextTokenizer {
                                                input_normalization: self
                                                    .loaded_pointer_head
                                                    .pointer_head
                                                    .input_normalization,
                                                model: self.loaded_llama_model.model.clone(),
                                            },
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
                                            desired_slots_total,
                                        ),
                                        llama_context,
                                        pending_requests: VecDeque::new(),
                                        scheduler_context: DecisionSchedulerContext {
                                            agent_name: self.inference_runtime_context.agent_name,
                                            n_batch,
                                            pointer_head: self.loaded_pointer_head.pointer_head,
                                        },
                                        scheduler_inbox: SchedulerInbox::new(scheduler_message_rx),
                                    }
                                    .run();
                                },
                            );
                        },
                    )
                    .map_err(DecisionError::from)
            })
        })
    }
}
