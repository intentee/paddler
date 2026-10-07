use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::channel;
use std::thread;

use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::model::params::LlamaModelParams;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use paddler_agent_pointer_head::pointer_head::PointerHead;
use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_agent_runtime::await_scheduler_startup::await_scheduler_startup;
use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_runtime::inference_thread_count::inference_thread_count;
use paddler_agent_runtime::llama_context_settings::LlamaContextSettings;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_agent_runtime::scheduler_request_preparer::SchedulerRequestPreparer;
use paddler_agent_runtime::scheduler_spawn_outcome::SchedulerSpawnOutcome;
use paddler_agent_runtime::send_startup_signal::send_startup_signal;
use paddler_agent_runtime::startup_signal_delivery::StartupSignalDelivery;
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
use crate::decision_request_preparer::DecisionRequestPreparer;
use crate::decision_scheduler::DecisionScheduler;
use crate::decision_scheduler_context::DecisionSchedulerContext;
use crate::decision_text_tokenizer::DecisionTextTokenizer;

const DECISION_ARCHITECTURES: [&str; 2] = ["qwen35", "qwen35moe"];
const DECISION_SLOTS_MINIMUM: u16 = 2;

pub struct DecisionPipeline {
    pub inference_runtime_context: InferenceRuntimeContext,
    pub model_path: PathBuf,
    pub model_runtime_parameters: ModelRuntimeParameters,
    pub pointer_head_path: PathBuf,
}

impl DecisionPipeline {
    pub async fn spawn(
        self,
        cancellation_token: &CancellationToken,
    ) -> Result<SchedulerSpawnOutcome<DecisionRequestPreparer, DecisionError>, DecisionError> {
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

    fn register_incompatibility(
        &self,
        incompatibility: PointerHeadIncompatibility,
    ) -> DecisionError {
        self.inference_runtime_context
            .slot_aggregated_status
            .register_issue(AgentIssue::PointerHeadIncompatibleWithModel(
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
                .register_issue(AgentIssue::PointerHeadCannotBeLoaded(ModelPath::from(
                    self.pointer_head_path.as_path(),
                )));

            DecisionError::PointerHeadCannotBeLoaded(source)
        })
    }

    fn require_decision_architecture(
        &self,
        loaded_llama_model: &LoadedLlamaModel,
    ) -> Result<(), DecisionError> {
        let architecture = loaded_llama_model
            .model
            .meta_val_str("general.architecture")
            .map_err(DecisionError::ModelArchitectureUnreadable)?;

        if DECISION_ARCHITECTURES.contains(&architecture.as_str()) {
            return Ok(());
        }

        self.inference_runtime_context
            .slot_aggregated_status
            .register_issue(AgentIssue::ModelArchitectureUnsupportedForDecisions(
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
            .register_issue(AgentIssue::SlotsInsufficientForDecisions(
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

        match DecisionDelimiterTokens::resolve(&loaded_llama_model.model, &pointer_head.delimiters)
        {
            Err(DecisionError::PointerHeadIncompatibleWithModel { incompatibility }) => {
                Err(self.register_incompatibility(incompatibility))
            }
            resolution => resolution,
        }
    }

    fn run_scheduler(
        self,
        agent_shutdown: &CancellationToken,
        scheduler_ready_tx: oneshot::Sender<Arc<SchedulerRequestPreparer<DecisionRequestPreparer>>>,
    ) -> Result<(), DecisionError> {
        let desired_slots_total = self.require_enough_slots()?;
        let pointer_head = self.load_pointer_head()?;
        let n_batch = self.model_runtime_parameters.n_batch.tokens_usize();
        let thread_count = inference_thread_count()?;
        let loaded_llama_model = LoadedLlamaModel::load(
            &self.inference_runtime_context,
            &self.model_path,
            &LlamaModelParams::default()
                .with_n_gpu_layers(self.model_runtime_parameters.n_gpu_layers),
        )?;

        self.inference_runtime_context
            .slot_aggregated_status
            .set_model_path(Some(ModelPath::from(self.model_path.as_path()).model_path));
        self.require_decision_architecture(&loaded_llama_model)?;

        let delimiter_tokens = self.resolve_delimiter_tokens(&loaded_llama_model, &pointer_head)?;
        let mut llama_context = loaded_llama_model.create_llama_context(
            &self.inference_runtime_context,
            LlamaContextSettings {
                model_runtime_parameters: &self.model_runtime_parameters,
                n_seq_max: u32::from(desired_slots_total),
                thread_count,
            }
            .into_llama_context_params()
            .with_kv_unified(true),
        )?;

        llama_context
            .enable_masked_nextn_embeddings()
            .map_err(DecisionError::HiddenStatesUnsupported)?;

        let mut batch =
            LlamaBatch::new(n_batch, 1).map_err(AgentRuntimeError::BatchAllocationFailed)?;

        loaded_llama_model.warm_up_llama_context(
            &mut llama_context,
            &mut batch,
            desired_slots_total,
        );

        let context_cells = llama_context.n_ctx();
        let (scheduler_message_tx, scheduler_message_rx) = channel();
        let request_preparer = Arc::new(SchedulerRequestPreparer {
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
        });

        if matches!(
            send_startup_signal(scheduler_ready_tx, request_preparer),
            StartupSignalDelivery::Abandoned
        ) {
            return Ok(());
        }

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

        Ok(())
    }
}
