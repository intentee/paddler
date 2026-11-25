#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_pipeline_harness::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_agent_pipeline_harness::llama_runtime::LlamaRuntime;
use paddler_agent_pipeline_harness::raw_prompt_generation::RawPromptGeneration;
use paddler_agent_runtime::prepares_scheduler_command::PreparesSchedulerCommand as _;
use paddler_agent_text_generation::text_generation_request::TextGenerationRequest;
use paddler_agent_text_generation_tests::first_message_content_request_preparer::FirstMessageContentRequestPreparer;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_model_card_fixture::qwen3_5_0_8b::qwen3_5_0_8b;
use paddler_model_card_fixture::served_context_size::served_context_size;

#[test]
fn a_prompt_that_tokenizes_to_nothing_refuses_the_generation() {
    let runtime = LlamaRuntime::load(
        &qwen3_5_0_8b()
            .cached_path()
            .expect("the Qwen3.5 model must be in the Hugging Face cache"),
        agent_inference_runtime_context(1),
    )
    .expect("the Qwen3.5 model must load");
    let preparer = FirstMessageContentRequestPreparer {
        llama_runtime: &runtime,
        sequence_context_size: served_context_size().get(),
    }
    .into_generation_request_preparer()
    .expect("the first message content chat template must be prepared");
    let RawPromptGeneration {
        agent_request,
        generate_tokens_stop_tx: _generate_tokens_stop_tx,
        mut generated_tokens_rx,
    } = RawPromptGeneration::new("", runtime.inference_runtime_context.slot_aggregated_status);

    let scheduler_command = preparer.prepare_scheduler_command(
        Some("test-agent"),
        TextGenerationRequest::ContinueFromRawPrompt(agent_request),
    );

    assert!(scheduler_command.is_none());
    assert!(matches!(
        generated_tokens_rx.try_recv(),
        Ok(GeneratedTokenResult::PromptTokenizedToNothing(_))
    ));
}
