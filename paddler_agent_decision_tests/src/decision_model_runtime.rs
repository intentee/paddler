use std::num::NonZeroU16;
use std::sync::Arc;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use llama_cpp_bindings_types::decision_request::DecisionRequest;
use llama_cpp_context::llama_batch::LlamaBatch;
use llama_cpp_decision::decision_model::DecisionModel;
use tokio::sync::mpsc;

use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_decision::decision_request_preparer::DecisionRequestPreparer;
use paddler_agent_decision::decision_scheduler::DecisionScheduler;
use paddler_agent_decision::decision_scheduler_context::DecisionSchedulerContext;
use paddler_agent_model_capabilities::established_model_capabilities::EstablishedModelCapabilities;
use paddler_agent_pipeline_harness::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_agent_pipeline_harness::established_decision_capabilities::EstablishedDecisionCapabilities;
use paddler_agent_pipeline_harness::llama_runtime::LlamaRuntime;
use paddler_agent_runtime::agent_request::AgentRequest;
use paddler_agent_runtime::prepares_scheduler_command::PreparesSchedulerCommand as _;
use paddler_agent_runtime::resident_llama_context::ResidentLlamaContext;
use paddler_agent_status::slot_guard::SlotGuard;
use paddler_messaging::request_params::decide_params::DecideParams;
use paddler_model_card_fixture::model_card::ModelCard;

use crate::decision_scheduler_run::DecisionSchedulerRun;
use crate::prepared_decision::PreparedDecision;

pub struct DecisionModelRuntime {
    pub decision_model: DecisionModel,
    pub model_capabilities: Arc<EstablishedModelCapabilities>,
    pub runtime: LlamaRuntime,
}

impl DecisionModelRuntime {
    /// # Errors
    ///
    /// Returns an error when the model is not cached or cannot load, when its capabilities cannot be established, and when it is not a decision model llama.cpp serves.
    pub fn load(model_card: &ModelCard, slots: u16) -> Result<Self> {
        let runtime = model_card
            .cached_path()
            .context("the decision model must be in the Hugging Face cache")
            .and_then(|model_path| {
                LlamaRuntime::load(&model_path, agent_inference_runtime_context(slots))
                    .context("the decision model must load")
            })?;
        let EstablishedDecisionCapabilities {
            decision_model,
            model_capabilities,
        } = EstablishedDecisionCapabilities::of(&runtime)?;

        Ok(Self {
            decision_model,
            model_capabilities,
            runtime,
        })
    }

    /// # Errors
    ///
    /// Returns the [`DecisionError`] of [`LlamaRuntime::create_decision_llama_context`] when the decision context cannot be created.
    pub fn create_llama_context(
        &self,
        sequences: u32,
    ) -> Result<ResidentLlamaContext<'_>, DecisionError> {
        self.runtime.create_decision_llama_context(
            &self.decision_model,
            &self.model_capabilities,
            sequences,
        )
    }

    /// Prepares `request` the way the decision pipeline does, holding `reserved_slots` of the agent and as many cache cells per slot as `cells_per_slot`.
    ///
    /// # Errors
    ///
    /// Returns an error holding the rejection the preparation reported when the request cannot be prepared.
    pub fn prepared_decision(
        &self,
        request: DecisionRequest,
        reserved_slots: NonZeroU16,
        cells_per_slot: u32,
    ) -> Result<PreparedDecision> {
        let (decision_result_tx, mut decision_result_rx) = mpsc::unbounded_channel();
        let (decision_stop_tx, decision_stop_rx) = mpsc::unbounded_channel();

        let scheduled_request = DecisionRequestPreparer {
            batch_tokens: self.runtime.n_batch_tokens(),
            cells_per_slot,
            decision_model: self.decision_model.clone(),
            model_capabilities: self.model_capabilities.clone(),
        }
        .prepare_scheduler_command(
            None,
            AgentRequest {
                params: DecideParams { request },
                response_tx: decision_result_tx,
                slot_guard: SlotGuard::new(
                    self.runtime
                        .inference_runtime_context
                        .slot_aggregated_status
                        .clone(),
                    reserved_slots,
                ),
                stop_rx: decision_stop_rx,
            },
        );

        match scheduled_request {
            Some(request) => Ok(PreparedDecision {
                decision_result_rx,
                decision_stop_tx,
                request,
            }),
            None => Err(anyhow!(
                "the decision must be prepared, but it was rejected with {:?}",
                decision_result_rx.try_recv()
            )),
        }
    }

    /// # Errors
    ///
    /// Returns the [`DecisionError`] of [`Self::create_llama_context`] when the decision context cannot be created.
    pub fn run_scheduler(
        &self,
        DecisionSchedulerRun {
            active_requests,
            agent_shutdown,
            capacity_ledger,
            pending_requests,
            scheduler_inbox,
            sequences,
        }: DecisionSchedulerRun,
    ) -> Result<(), DecisionError> {
        self.create_llama_context(u32::from(sequences))
            .map(|llama_context| {
                DecisionScheduler {
                    active_requests,
                    agent_shutdown,
                    batch: LlamaBatch::new(llama_context.limits()),
                    capacity_ledger,
                    llama_context,
                    pending_requests,
                    scheduler_context: DecisionSchedulerContext {
                        agent_name: None,
                        model_capabilities: self.model_capabilities.clone(),
                    },
                    scheduler_inbox,
                }
                .run();
            })
    }
}
