#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings::batch_add_error::BatchAddError;
use llama_cpp_bindings::context::context_limits::ContextLimits;
use llama_cpp_bindings::error::token_outside_vocabulary::TokenOutsideVocabulary;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::token::LlamaToken;
use tokio::sync::mpsc::unbounded_channel;
use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::scheduler_message::SchedulerMessage;
use paddler_agent_text_generation::continuous_batch_scheduler::ContinuousBatchScheduler;
use paddler_agent_text_generation::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use paddler_agent_text_generation::continuous_batch_scheduler_params::ContinuousBatchSchedulerParams;
use paddler_agent_text_generation::prepared_generation_request::PreparedGenerationRequest;
use paddler_agent_text_generation::prepared_prompt::PreparedPrompt;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::first_message_content_request_preparer::FirstMessageContentRequestPreparer;
use paddler_tests::kev_checkpoint::KevCheckpoint;
use paddler_tests::last_generated_token_result::last_generated_token_result;
use paddler_tests::llama_runtime::LlamaRuntime;
use paddler_tests::raw_prompt_generation::RawPromptGeneration;

#[test]
fn a_prompt_token_outside_the_vocabulary_fails_the_generation_as_batch_assembly() {
    let runtime = LlamaRuntime::load(
        &KevCheckpoint::Kev0_8b.target_path("model.gguf"),
        agent_inference_runtime_context(1),
    )
    .expect("the kev model must load");
    let llama_context = runtime
        .create_text_generation_llama_context(1)
        .expect("the context must be created");
    let sequence_context_size = llama_context.n_ctx_seq();
    let model = &runtime.loaded_llama_model.model;
    let token_outside_the_vocabulary = LlamaToken::new(model.n_vocab());
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
            prompt: PreparedPrompt::TextTokens(vec![token_outside_the_vocabulary]),
            ..FirstMessageContentRequestPreparer {
                llama_runtime: &runtime,
                sequence_context_size,
            }
            .into_generation_request_preparer()
            .expect("the first message content chat template must be prepared")
            .prepare_raw_prompt(agent_request)
            .expect("the generation must be prepared")
        }))
        .expect("the scheduler must accept the generation");
    scheduler_message_tx
        .send(SchedulerMessage::Shutdown)
        .expect("the scheduler must accept the shutdown");

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
        Some(GeneratedTokenResult::BatchAssemblyFailed(
            TextGenerationError::BatchAssemblyFailed(BatchAddError::TokenOutsideVocabulary(
                TokenOutsideVocabulary {
                    token: token_outside_the_vocabulary,
                    n_vocab: model.n_vocab(),
                }
            ))
            .to_string()
        ))
    );
}
