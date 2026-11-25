use std::sync::Arc;
use std::time::Instant;

use llama_cpp_decision::decision_model::DecisionModel;
use llama_cpp_decision::decision_prompt::DecisionPrompt;
use llama_cpp_decision::decision_prompts::DecisionPrompts;
use llama_cpp_decision::prepared_decision_request::PreparedDecisionRequest;

use paddler_agent_model_capabilities::established_model_capabilities::EstablishedModelCapabilities;
use paddler_agent_runtime::agent_request::AgentRequest;
use paddler_agent_runtime::prepares_scheduler_command::PreparesSchedulerCommand;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::oversized_decision_details::OversizedDecisionDetails;
use paddler_messaging::request_params::decide_params::DecideParams;

use crate::decision_error::DecisionError;
use crate::decision_plan::DecisionPlan;
use crate::reports_decision_outcome::ReportsDecisionOutcome as _;
use crate::scheduled_decision_request::ScheduledDecisionRequest;

pub struct DecisionRequestPreparer {
    pub batch_tokens: usize,
    pub cells_per_slot: u32,
    pub decision_model: DecisionModel,
    pub model_capabilities: Arc<EstablishedModelCapabilities>,
}

impl DecisionRequestPreparer {
    fn plan(&self, DecideParams { request }: &DecideParams) -> Result<DecisionPlan, DecisionError> {
        let prepared = PreparedDecisionRequest::new(&self.decision_model, request)
            .map_err(DecisionError::RequestRejected)?;
        let DecisionPrompts {
            prompts,
            shared_prefix_length,
        } = self
            .model_capabilities
            .projector
            .with_multimodal_context(|projector| prepared.prompts(projector))
            .map_err(DecisionError::PromptsRejected)?;

        for prompt in &prompts {
            self.require_prompt_fits_its_batch(prompt)?;
        }

        Ok(DecisionPlan {
            prepared,
            prompts,
            shared_prefix_length,
        })
    }

    fn prepare(
        &self,
        AgentRequest {
            params,
            response_tx: decision_result_tx,
            slot_guard,
            stop_rx: decision_stop_rx,
        }: AgentRequest<DecideParams, DecisionResult>,
    ) -> Result<ScheduledDecisionRequest, DecisionError> {
        let started_at = Instant::now();
        let reserved_context_size =
            u32::from(slot_guard.slots().get()).checked_mul(self.cells_per_slot);
        let plan = self.plan(&params)?;
        let required_tokens = plan.required_cells();

        if let Some(context_size) = reserved_context_size
            && required_tokens > context_size as usize
        {
            return Err(DecisionError::RequestExceedsContext {
                details: OversizedDecisionDetails {
                    context_size,
                    required_tokens,
                },
            });
        }

        Ok(ScheduledDecisionRequest {
            decision_result_tx,
            decision_stop_rx,
            plan,
            slot_guard,
            started_at,
        })
    }

    fn require_prompt_fits_its_batch(&self, prompt: &DecisionPrompt) -> Result<(), DecisionError> {
        let prompt_tokens = prompt.tokens().len();

        if prompt.must_be_decoded_at_once() && prompt_tokens > self.batch_tokens {
            return Err(DecisionError::PromptExceedsBatch {
                batch_tokens: self.batch_tokens,
                prompt_tokens,
            });
        }

        self.require_read_span_fits_a_batch(prompt_tokens, prompt.first_read_position())
    }

    const fn require_read_span_fits_a_batch(
        &self,
        prompt_tokens: usize,
        first_read_position: usize,
    ) -> Result<(), DecisionError> {
        let read_tokens = prompt_tokens.saturating_sub(first_read_position);

        if read_tokens > self.batch_tokens {
            return Err(DecisionError::PromptReadSpanExceedsBatch {
                batch_tokens: self.batch_tokens,
                read_tokens,
            });
        }

        Ok(())
    }
}

impl PreparesSchedulerCommand for DecisionRequestPreparer {
    type Command = ScheduledDecisionRequest;
    type Request = AgentRequest<DecideParams, DecisionResult>;

    fn prepare_scheduler_command(
        &self,
        agent_name: Option<&str>,
        request: Self::Request,
    ) -> Option<Self::Command> {
        let decision_result_tx = request.response_tx.clone();

        match self.prepare(request) {
            Ok(prepared) => Some(prepared),
            Err(rejection) => {
                rejection.report(agent_name, &decision_result_tx);

                None
            }
        }
    }
}
