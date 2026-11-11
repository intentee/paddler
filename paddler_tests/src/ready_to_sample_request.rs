use std::slice;

use anyhow::Context as _;
use anyhow::Result;
use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::token::data_array::LlamaTokenDataArray;
use log::LevelFilter;
use log::set_max_level;

use paddler_agent_runtime::sequence_id_guard::SequenceIdGuard;
use paddler_agent_runtime::sequence_id_pool::SequenceIdPool;
use paddler_agent_text_generation::continuous_batch_active_request::ContinuousBatchActiveRequest;
use paddler_agent_text_generation::continuous_batch_generation_step::ContinuousBatchGenerationStep;
use paddler_agent_text_generation::continuous_batch_request_phase::ContinuousBatchRequestPhase;
use paddler_agent_text_generation::continuous_batch_request_state::ContinuousBatchRequestState;
use paddler_agent_text_generation::continuous_batch_scheduler::advance_generating_phase::AdvanceGeneratingPhase;
use paddler_agent_text_generation::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use paddler_agent_text_generation::continuous_batch_terminal_outcome::ContinuousBatchTerminalOutcome;
use paddler_agent_text_generation::prepared_generation_request::PreparedGenerationRequest;

use crate::first_message_content_request_preparer::FirstMessageContentRequestPreparer;
use crate::llama_runtime::LlamaRuntime;
use crate::prompt_sampling_positions::PromptSamplingPositions;
use crate::raw_prompt_generation::RawPromptGeneration;

fn ready_to_sample(
    PreparedGenerationRequest {
        generate_tokens_stop_rx,
        generated_tokens_tx,
        max_tokens,
        slot_guard,
        token_classification,
        token_sampling,
        tool_call_handling,
        ..
    }: PreparedGenerationRequest,
    model: &LlamaModel,
    batch_position: i32,
    next_token_position: i32,
) -> Result<ContinuousBatchActiveRequest<'_>> {
    token_classification
        .into_classifier(model)
        .context("the answer classifier must start")
        .and_then(|token_classifier| {
            SequenceIdGuard::acquire(&SequenceIdPool::new(1))
                .context("a fresh pool must hand out a sequence")
                .map(|sequence_id_guard| ContinuousBatchActiveRequest {
                    state: ContinuousBatchRequestState {
                        current_token_position: next_token_position,
                        last_outcome_section: token_classifier.current_section(),
                        max_tokens,
                        phase: ContinuousBatchRequestPhase::Generating(
                            ContinuousBatchGenerationStep::ReadyToSample { batch_position },
                        ),
                        sampled_tokens: 0,
                        unread_prompt_tokens: Vec::new().into_iter(),
                    },
                    token_classifier,
                    token_sampling,
                    generated_tokens_tx,
                    generate_tokens_stop_rx,
                    sequence_id_guard,
                    slot_guard,
                    tool_call_handling,
                })
        })
}

pub struct ReadyToSampleRequest<'runtime> {
    pub llama_context: &'runtime LlamaContext<'runtime>,
    pub prompt: &'static str,
    pub prompt_sampling_positions: PromptSamplingPositions,
    pub runtime: &'runtime LlamaRuntime,
}

impl ReadyToSampleRequest<'_> {
    /// # Errors
    ///
    /// Returns an error when the generation cannot be prepared, when its answer classifier cannot start, and when no sequence is free for it.
    pub fn advance(
        self,
        customize_request: impl FnOnce(PreparedGenerationRequest) -> PreparedGenerationRequest,
    ) -> Result<ContinuousBatchTerminalOutcome> {
        let Self {
            llama_context,
            prompt,
            prompt_sampling_positions:
                PromptSamplingPositions {
                    batch_position,
                    next_token_position,
                },
            runtime,
        } = self;

        set_max_level(LevelFilter::Trace);

        let sequence_context_size = llama_context.n_ctx_seq();
        let RawPromptGeneration {
            agent_request,
            generate_tokens_stop_tx: _generate_tokens_stop_tx,
            generated_tokens_rx: _generated_tokens_rx,
        } = RawPromptGeneration::new(
            prompt,
            runtime
                .inference_runtime_context
                .slot_aggregated_status
                .clone(),
        );

        FirstMessageContentRequestPreparer {
            llama_runtime: runtime,
            sequence_context_size,
        }
        .into_generation_request_preparer()
        .and_then(|generation_request_preparer| {
            generation_request_preparer
                .prepare_raw_prompt(agent_request)
                .context("the generation must be prepared")
        })
        .and_then(|prepared_generation_request| {
            ready_to_sample(
                customize_request(prepared_generation_request),
                &runtime.loaded_llama_model.model,
                batch_position,
                next_token_position,
            )
        })
        .map(|mut active_request| {
            AdvanceGeneratingPhase {
                candidates: &mut LlamaTokenDataArray::new(Vec::new(), false),
                ingest_outcomes: &mut Vec::new(),
                llama_context,
                scheduler_context: &ContinuousBatchSchedulerContext {
                    agent_name: None,
                    desired_slots_total: 1,
                    model: &runtime.loaded_llama_model.model,
                    sequence_context_size,
                },
            }
            .run(slice::from_mut(&mut active_request));

            active_request.state.into_terminal_outcome()
        })
    }
}
