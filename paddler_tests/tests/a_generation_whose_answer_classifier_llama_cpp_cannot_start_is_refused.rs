#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings::chat_templates::ChatTemplates;
use llama_cpp_bindings::context::context_limits::ContextLimits;
use llama_cpp_bindings::error::parse_chat_message_error::ParseChatMessageError;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings_sys::LLAMA_RS_FAULT_POINT_CHAT_SESSION_READ_THROWS;
use llama_cpp_fault_injection::armed_fault::ArmedFault;
use llama_cpp_fault_injection::injected_fault_message::injected_fault_message;
use tokio::sync::mpsc::unbounded_channel;
use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::scheduler_message::SchedulerMessage;
use paddler_agent_text_generation::continuous_batch_scheduler::ContinuousBatchScheduler;
use paddler_agent_text_generation::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use paddler_agent_text_generation::continuous_batch_scheduler_params::ContinuousBatchSchedulerParams;
use paddler_agent_text_generation::prepared_generation_request::PreparedGenerationRequest;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_agent_text_generation::token_classification::TokenClassification;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::last_generated_token_result::last_generated_token_result;
use paddler_tests::llama_runtime::LlamaRuntime;
use paddler_tests::raw_prompt_generation::RawPromptGeneration;
use paddler_tests::weather_chat_session::weather_chat_session;

use crate::first_message_content_request_preparer::FirstMessageContentRequestPreparer;

#[test]
fn a_generation_whose_answer_classifier_llama_cpp_cannot_start_is_refused() {
    let runtime = LlamaRuntime::load(
        &qwen3_0_6b()
            .cached_path()
            .expect("the model must be in the Hugging Face cache"),
        agent_inference_runtime_context(1),
    )
    .expect("the model must load");
    let llama_context = runtime
        .create_text_generation_llama_context(1)
        .expect("the context must be created");
    let sequence_context_size = llama_context.n_ctx_seq();
    let model = &runtime.loaded_llama_model.model;
    let chat_templates = ChatTemplates::new(model).expect("the model must embed a chat template");
    let RawPromptGeneration {
        agent_request,
        generate_tokens_stop_tx: _generate_tokens_stop_tx,
        generated_tokens_rx,
    } = RawPromptGeneration::new(
        "Paddler",
        runtime
            .inference_runtime_context
            .slot_aggregated_status
            .clone(),
    );
    let (scheduler_message_tx, scheduler_message_rx) = unbounded_channel();

    scheduler_message_tx
        .send(SchedulerMessage::Command(PreparedGenerationRequest {
            token_classification: TokenClassification::ChatSession(
                weather_chat_session(model, &chat_templates)
                    .expect("the model's template must serve the weather question"),
            ),
            ..FirstMessageContentRequestPreparer {
                llama_runtime: &runtime,
                sequence_context_size,
            }
            .into_generation_request_preparer()
            .prepare_raw_prompt(agent_request)
            .expect("the generation must be prepared")
        }))
        .expect("the scheduler must accept the generation");
    scheduler_message_tx
        .send(SchedulerMessage::Shutdown)
        .expect("the scheduler must accept the shutdown");

    let _classifier_cannot_start = ArmedFault::at(LLAMA_RS_FAULT_POINT_CHAT_SESSION_READ_THROWS);

    ContinuousBatchScheduler::new(ContinuousBatchSchedulerParams {
        agent_shutdown: CancellationToken::new(),
        batch: LlamaBatch::new(ContextLimits::of(&llama_context)),
        llama_context,
        scheduler_context: ContinuousBatchSchedulerContext {
            agent_name: None,
            desired_slots_total: 1,
            model,
            sequence_context_size,
        },
        scheduler_message_rx,
    })
    .run();

    assert_eq!(
        last_generated_token_result(generated_tokens_rx),
        Some(
            TextGenerationError::AnswerClassifierUnavailable(ParseChatMessageError::Reported {
                message: injected_fault_message(),
            })
            .into_generated_token_result(None)
        )
    );
}
