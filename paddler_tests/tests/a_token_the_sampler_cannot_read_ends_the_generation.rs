#![cfg(feature = "tests_that_use_llms")]

use std::slice;
use std::sync::Arc;

use llama_cpp_bindings::SampledTokenClassifier;
use llama_cpp_bindings::error::LogitsError;
use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::token::data_array::LlamaTokenDataArray;

use paddler_agent_runtime::sequence_id_guard::SequenceIdGuard;
use paddler_agent_runtime::sequence_id_pool::SequenceIdPool;
use paddler_agent_text_generation::chat_template_renderer::ChatTemplateRenderer;
use paddler_agent_text_generation::continuous_batch_active_request::ContinuousBatchActiveRequest;
use paddler_agent_text_generation::continuous_batch_generation_step::ContinuousBatchGenerationStep;
use paddler_agent_text_generation::continuous_batch_request_phase::ContinuousBatchRequestPhase;
use paddler_agent_text_generation::continuous_batch_request_state::ContinuousBatchRequestState;
use paddler_agent_text_generation::continuous_batch_scheduler::advance_generating_phase::AdvanceGeneratingPhase;
use paddler_agent_text_generation::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use paddler_agent_text_generation::continuous_batch_terminal_outcome::ContinuousBatchTerminalOutcome;
use paddler_agent_text_generation::json_tools_serializer::JsonToolsSerializer;
use paddler_agent_text_generation::prepared_generation_request::PreparedGenerationRequest;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_agent_text_generation::token_classification::TokenClassification;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::first_message_content_chat_template::first_message_content_chat_template;
use paddler_tests::kev_0_8b_target_path::kev_0_8b_target_path;
use paddler_tests::llama_runtime::LlamaRuntime;
use paddler_tests::raw_prompt_generation::RawPromptGeneration;
use paddler_tests::text_only_generation_parts::TextOnlyGenerationParts;

#[test]
fn a_token_the_sampler_cannot_read_ends_the_generation() {
    let runtime = LlamaRuntime::load(
        &kev_0_8b_target_path("model.gguf"),
        agent_inference_runtime_context(1),
    )
    .expect("the kev model must load");
    let llama_context = runtime
        .create_text_generation_llama_context(1)
        .expect("the context must be created");
    let sequence_context_size = llama_context.n_ctx_seq();
    let classifier_model: &'static LlamaModel =
        Box::leak(Box::new(Arc::clone(&runtime.loaded_llama_model.model)));
    let RawPromptGeneration {
        agent_request,
        generate_tokens_stop_tx: _generate_tokens_stop_tx,
        generated_tokens_rx: _generated_tokens_rx,
    } = RawPromptGeneration::new(
        "Paddler",
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
    } = runtime
        .text_only_generation_request_preparer(
            TextOnlyGenerationParts {
                chat_template_renderer: ChatTemplateRenderer::new(
                    first_message_content_chat_template(),
                )
                .expect("the chat template must compile"),
                sequence_context_size,
                streaming_markers: runtime
                    .loaded_llama_model
                    .model
                    .streaming_markers()
                    .expect("the streaming markers must be detected"),
            },
            runtime.sampler_chain_factory(),
            JsonToolsSerializer,
        )
        .prepare_raw_prompt(agent_request)
        .expect("the generation must be prepared");
    let token_classifier =
        SampledTokenClassifier::new(classifier_model, streaming_markers, bare_json_tool_calls);
    let mut active_request = ContinuousBatchActiveRequest {
        state: ContinuousBatchRequestState {
            current_token_position: 0,
            last_outcome_section: token_classifier.current_section(),
            max_tokens,
            phase: ContinuousBatchRequestPhase::Generating(
                ContinuousBatchGenerationStep::ReadyToSample { batch_position: 0 },
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
        llama_context: &llama_context,
        scheduler_context: &ContinuousBatchSchedulerContext {
            agent_name: None,
            desired_slots_total: 1,
            model: runtime.loaded_llama_model.model.clone(),
            n_batch: runtime.model_runtime_parameters.n_batch,
            sequence_context_size,
        },
    }
    .run(slice::from_mut(&mut active_request));

    assert_eq!(
        active_request.state.into_terminal_outcome(),
        ContinuousBatchTerminalOutcome::EmitToClient(GeneratedTokenResult::SamplerError(
            TextGenerationError::TokenDataUnreadable(LogitsError::TokenNotInitialized(0))
                .to_string()
        ))
    );
}
