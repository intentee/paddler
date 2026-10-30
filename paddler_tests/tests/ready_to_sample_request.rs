#![cfg(feature = "tests_that_use_llms")]

use std::slice;
use std::sync::Arc;

use llama_cpp_bindings::SampledTokenClassifier;
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
use paddler_agent_text_generation::token_classification::TokenClassification;
use paddler_tests::llama_runtime::LlamaRuntime;
use paddler_tests::raw_prompt_generation::RawPromptGeneration;

use crate::first_message_content_request_preparer::FirstMessageContentRequestPreparer;

pub struct ReadyToSampleRequest<'runtime> {
    pub batch_position: i32,
    pub llama_context: &'runtime LlamaContext<'runtime>,
    pub prompt: &'static str,
    pub runtime: &'runtime LlamaRuntime,
}

impl ReadyToSampleRequest<'_> {
    pub fn advance(
        self,
        customize_request: impl FnOnce(PreparedGenerationRequest) -> PreparedGenerationRequest,
    ) -> ContinuousBatchTerminalOutcome {
        let Self {
            batch_position,
            llama_context,
            prompt,
            runtime,
        } = self;

        set_max_level(LevelFilter::Trace);

        let sequence_context_size = llama_context.n_ctx_seq();
        let classifier_model: &'static LlamaModel =
            Box::leak(Box::new(Arc::clone(&runtime.loaded_llama_model.model)));
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
        let PreparedGenerationRequest {
            generate_tokens_stop_rx,
            generated_tokens_tx,
            max_tokens,
            slot_guard,
            token_classification:
                TokenClassification {
                    bare_json_tool_calls,
                    streaming_markers,
                },
            token_sampling,
            tool_call_handling,
            ..
        } = customize_request(
            FirstMessageContentRequestPreparer {
                llama_runtime: runtime,
                sequence_context_size,
            }
            .into_generation_request_preparer()
            .prepare_raw_prompt(agent_request)
            .expect("the generation must be prepared"),
        );
        let token_classifier =
            SampledTokenClassifier::new(classifier_model, streaming_markers, bare_json_tool_calls);
        let mut active_request = ContinuousBatchActiveRequest {
            state: ContinuousBatchRequestState {
                current_token_position: batch_position + 1,
                last_outcome_section: token_classifier.current_section(),
                max_tokens,
                phase: ContinuousBatchRequestPhase::Generating(
                    ContinuousBatchGenerationStep::ReadyToSample { batch_position },
                ),
                prompt_tokens: Vec::new(),
                prompt_tokens_ingested: 0,
                sampled_tokens: 0,
            },
            token_classifier,
            token_sampling,
            generated_tokens_tx,
            generate_tokens_stop_rx,
            sequence_id_guard: SequenceIdGuard::acquire(&SequenceIdPool::new(1))
                .expect("a fresh pool must hand out a sequence"),
            slot_guard,
            tool_call_handling,
        };

        AdvanceGeneratingPhase {
            candidates: &mut LlamaTokenDataArray::new(Vec::new(), false),
            ingest_outcomes: &mut Vec::new(),
            llama_context,
            scheduler_context: &ContinuousBatchSchedulerContext {
                agent_name: None,
                desired_slots_total: 1,
                model: &runtime.loaded_llama_model.model,
                n_batch: runtime.model_runtime_parameters.n_batch,
                sequence_context_size,
            },
        }
        .run(slice::from_mut(&mut active_request));

        active_request.state.into_terminal_outcome()
    }
}
