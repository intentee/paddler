use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::channel;

use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::model::params::LlamaModelParams;
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use paddler_agent_pointer_head::pointer_head::PointerHead;
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
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_architecture_unsupported_for_decisions_params::ModelArchitectureUnsupportedForDecisionsParams;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::agent_issue_params::pointer_head_incompatibility::PointerHeadIncompatibility;
use paddler_messaging::agent_issue_params::pointer_head_incompatible_with_model_params::PointerHeadIncompatibleWithModelParams;
use paddler_messaging::agent_issue_params::slots_insufficient_for_decisions_params::SlotsInsufficientForDecisionsParams;

use crate::decision_capacity_ledger::DecisionCapacityLedger;
use crate::decision_delimiter_tokens::DecisionDelimiterTokens;
use crate::decision_error::DecisionError;
use crate::decision_llama_context_settings::DecisionLlamaContextSettings;
use crate::decision_request_preparer::DecisionRequestPreparer;
use crate::decision_scheduler::DecisionScheduler;
use crate::decision_scheduler_context::DecisionSchedulerContext;
use crate::decision_slots_minimum::DECISION_SLOTS_MINIMUM;
use crate::decision_text_tokenizer::DecisionTextTokenizer;
use crate::general_architecture_metadata::GeneralArchitectureMetadata;
use crate::reads_model_architecture::ReadsModelArchitecture;

const DECISION_ARCHITECTURES: [&str; 2] = ["qwen35", "qwen35moe"];

pub struct DecisionPipeline<TArchitecture: ReadsModelArchitecture = GeneralArchitectureMetadata> {
    pub architecture: TArchitecture,
    pub inference_runtime_context: InferenceRuntimeContext,
    pub model_path: PathBuf,
    pub model_runtime_parameters: ModelRuntimeParameters,
    pub pointer_head_path: PathBuf,
}

impl<TArchitecture: ReadsModelArchitecture> DecisionPipeline<TArchitecture> {
    pub async fn spawn(
        self,
        cancellation_token: &CancellationToken,
    ) -> Result<SchedulerSpawnOutcome<DecisionRequestPreparer>, DecisionError> {
        start_scheduler_thread(
            cancellation_token,
            move |agent_shutdown, scheduler_startup_tx| {
                self.run_scheduler(agent_shutdown, scheduler_startup_tx)
            },
        )
        .await
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
                    pointer_head_path: ModelPath::from(self.pointer_head_path.as_path()),
                },
            ));

        DecisionError::PointerHeadIncompatibleWithModel { incompatibility }
    }

    fn load_pointer_head(&self) -> Result<PointerHead, DecisionError> {
        PointerHead::load(&self.pointer_head_path).map_err(|source| {
            self.inference_runtime_context
                .slot_aggregated_status
                .register_pipeline_issue(AgentIssue::PointerHeadCannotBeLoaded(ModelPath::from(
                    self.pointer_head_path.as_path(),
                )));

            DecisionError::PointerHeadCannotBeLoaded(source)
        })
    }

    fn require_decision_architecture(
        &self,
        loaded_llama_model: &LoadedLlamaModel,
    ) -> Result<(), DecisionError> {
        let architecture = self
            .architecture
            .model_architecture(&loaded_llama_model.model)
            .map_err(DecisionError::ModelArchitectureUnreadable)?;

        if DECISION_ARCHITECTURES.contains(&architecture.as_str()) {
            return Ok(());
        }

        self.inference_runtime_context
            .slot_aggregated_status
            .register_pipeline_issue(AgentIssue::ModelArchitectureUnsupportedForDecisions(
                ModelArchitectureUnsupportedForDecisionsParams {
                    architecture: architecture.clone(),
                    model_path: ModelPath::from(self.model_path.as_path()),
                },
            ));

        Err(DecisionError::ModelArchitectureUnsupported { architecture })
    }

    fn require_enough_slots(&self) -> Result<u16, DecisionError> {
        let desired_slots = self
            .inference_runtime_context
            .slot_aggregated_status
            .desired_slots_total;

        if desired_slots >= DECISION_SLOTS_MINIMUM {
            return Ok(desired_slots);
        }

        self.inference_runtime_context
            .slot_aggregated_status
            .register_pipeline_issue(AgentIssue::SlotsInsufficientForDecisions(
                SlotsInsufficientForDecisionsParams {
                    desired_slots,
                    required_slots: DECISION_SLOTS_MINIMUM,
                },
            ));

        Err(DecisionError::SlotsInsufficient {
            desired_slots,
            required_slots: DECISION_SLOTS_MINIMUM,
        })
    }

    fn resolve_delimiter_tokens(
        &self,
        loaded_llama_model: &LoadedLlamaModel,
        pointer_head: &PointerHead,
    ) -> Result<DecisionDelimiterTokens, DecisionError> {
        let model_hidden_size = loaded_llama_model.model.n_embd() as usize;

        if pointer_head.hidden_size() != model_hidden_size {
            return Err(self.register_incompatibility(
                PointerHeadIncompatibility::HiddenSizeMismatch {
                    model_hidden_size,
                    pointer_head_hidden_size: pointer_head.hidden_size(),
                },
            ));
        }

        match DecisionDelimiterTokens::resolve(
            loaded_llama_model.model.as_ref(),
            &pointer_head.delimiters,
        ) {
            Err(DecisionError::PointerHeadIncompatibleWithModel { incompatibility }) => {
                Err(self.register_incompatibility(incompatibility))
            }
            resolution => resolution,
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
            let desired_slots_total = self.require_enough_slots()?;
            let pointer_head = self.load_pointer_head()?;
            let n_batch = self.model_runtime_parameters.n_batch.tokens_usize();
            let thread_count = inference_thread_count(
                self.inference_runtime_context
                    .available_parallelism
                    .as_ref(),
            )?;
            let loaded_llama_model = LoadedLlamaModel::load(
                &self.inference_runtime_context,
                &self.model_path,
                &LlamaModelParams::default()
                    .with_n_gpu_layers(self.model_runtime_parameters.n_gpu_layers),
            )?;

            continue_startup_unless_shutting_down(agent_shutdown, || {
                self.require_decision_architecture(&loaded_llama_model)?;

                let delimiter_tokens =
                    self.resolve_delimiter_tokens(&loaded_llama_model, &pointer_head)?;
                let mut llama_context = DecisionLlamaContextSettings {
                    llama_context_settings: LlamaContextSettings {
                        model_runtime_parameters: &self.model_runtime_parameters,
                        n_seq_max: u32::from(desired_slots_total),
                        thread_count,
                    },
                }
                .create_llama_context(&loaded_llama_model, &self.inference_runtime_context)?;

                let mut batch =
                    LlamaBatch::new(self.model_runtime_parameters.n_batch.capacity(), 1);

                continue_startup_unless_shutting_down(agent_shutdown, || {
                    loaded_llama_model.warm_up_llama_context(
                        &mut llama_context,
                        &mut batch,
                        desired_slots_total,
                    );

                    let context_cells = llama_context.n_ctx();
                    let (scheduler_message_tx, scheduler_message_rx) = channel();

                    run_scheduler_unless_startup_abandoned(
                        scheduler_startup_tx,
                        Ok(SchedulerReady {
                            model_metadata: loaded_llama_model.model_metadata.clone(),
                            request_preparer: Arc::new(SchedulerRequestPreparer {
                                agent_name: self.inference_runtime_context.agent_name.clone(),
                                preparation: DecisionRequestPreparer {
                                    context_cells,
                                    delimiter_tokens,
                                    text_tokenizer: DecisionTextTokenizer {
                                        loaded_llama_model: loaded_llama_model.clone(),
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
                                    context_cells as usize,
                                    desired_slots_total,
                                ),
                                llama_context,
                                pending_requests: VecDeque::new(),
                                scheduler_context: DecisionSchedulerContext {
                                    agent_name: self.inference_runtime_context.agent_name,
                                    n_batch,
                                    pointer_head,
                                },
                                scheduler_message_rx,
                                shutdown_requested: false,
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
