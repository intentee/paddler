use std::collections::VecDeque;
use std::num::NonZeroU16;
use std::num::NonZeroU32;
use std::sync::Arc;

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
use paddler_messaging::agent_issue::AgentIssue;

use crate::constants::KEV_DECISION_DELIMITERS;
use crate::decision_capacity_ledger::DecisionCapacityLedger;
use crate::decision_delimiter_tokens::DecisionDelimiterTokens;
use crate::decision_error::DecisionError;
use crate::decision_llama_context_settings::DecisionLlamaContextSettings;
use crate::decision_request_preparer::DecisionRequestPreparer;
use crate::decision_scheduler::DecisionScheduler;
use crate::decision_scheduler_context::DecisionSchedulerContext;
use crate::decision_text_tokenizer::DecisionTextTokenizer;
use crate::kev_option_scorer::KevOptionScorer;

pub struct DecisionPipeline {
    pub decision_slots: NonZeroU16,
    pub inference_runtime_context: InferenceRuntimeContext,
    pub loaded_llama_model: LoadedLlamaModel,
    pub model_runtime_parameters: ModelRuntimeParameters,
}

impl DecisionPipeline {
    /// # Errors
    ///
    /// Returns [`DecisionError::KevArchitectureUnknown`], [`DecisionError::KevHeadDimensionUnrepresentable`], [`DecisionError::KevTemperatureMissing`], [`DecisionError::KevTemperatureUnparseable`] or [`DecisionError::KevTemperatureOutOfRange`] when the model cannot score options, [`DecisionError::DelimiterIsNotAControlToken`], [`DecisionError::DelimiterTokenizationFailed`] or [`DecisionError::DelimiterAttributesUnreadable`] when a kev delimiter cannot be resolved against the vocabulary, and [`DecisionError::AgentRuntime`] when the decision context cannot be created or warmed up.
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

    fn register_malformed_decision_model(&self, defect: DecisionError) -> DecisionError {
        self.inference_runtime_context
            .slot_aggregated_status
            .register_pipeline_issue(AgentIssue::DecisionModelIsMalformed(
                self.loaded_llama_model
                    .llama_model_settings
                    .reported_model_path(),
            ));

        defect
    }

    fn resolve_delimiter_tokens(&self) -> Result<DecisionDelimiterTokens, DecisionError> {
        DecisionDelimiterTokens::resolve(
            &*self.loaded_llama_model.model.llama_model,
            &KEV_DECISION_DELIMITERS,
        )
        .map_err(|resolution_error| self.register_malformed_decision_model(resolution_error))
    }

    fn resolve_option_scorer(&self) -> Result<KevOptionScorer, DecisionError> {
        KevOptionScorer::of(
            &self.loaded_llama_model.model_metadata,
            self.loaded_llama_model.model.n_embd_out(),
        )
        .map_err(|defect| self.register_malformed_decision_model(defect))
    }

    fn run_scheduler(
        self,
        agent_shutdown: &CancellationToken,
        scheduler_startup_tx: &UnboundedSender<
            Result<SchedulerReady<DecisionRequestPreparer>, DecisionError>,
        >,
    ) -> Result<(), DecisionError> {
        continue_startup_unless_shutting_down(agent_shutdown, || {
            let option_scorer = self.resolve_option_scorer()?;
            let delimiter_tokens = self.resolve_delimiter_tokens()?;
            let decision_slots = self.decision_slots;
            let llama_context = DecisionLlamaContextSettings {
                llama_context_settings: self.loaded_llama_model.llama_context_settings(
                    &self.model_runtime_parameters,
                    u32::from(decision_slots.get()),
                ),
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
                                        cells_per_slot,
                                        delimiter_tokens,
                                        text_tokenizer: DecisionTextTokenizer {
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
                                        decision_slots.get(),
                                    ),
                                    llama_context,
                                    pending_requests: VecDeque::new(),
                                    scheduler_context: DecisionSchedulerContext {
                                        agent_name: self.inference_runtime_context.agent_name,
                                        option_scorer,
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
