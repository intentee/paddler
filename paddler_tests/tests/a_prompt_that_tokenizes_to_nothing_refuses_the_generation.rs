#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_runtime::prepares_scheduler_command::PreparesSchedulerCommand as _;
use paddler_agent_text_generation::text_generation_request::TextGenerationRequest;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::kev_checkpoint::KevCheckpoint;
use paddler_tests::llama_runtime::LlamaRuntime;
use paddler_tests::raw_prompt_generation::RawPromptGeneration;

use crate::first_message_content_request_preparer::FirstMessageContentRequestPreparer;

#[test]
fn a_prompt_that_tokenizes_to_nothing_refuses_the_generation() {
    let runtime = LlamaRuntime::load(
        &KevCheckpoint::Kev0_8b.target_path("model.gguf"),
        agent_inference_runtime_context(1),
    )
    .expect("the kev model must load");
    let preparer = FirstMessageContentRequestPreparer {
        llama_runtime: &runtime,
        sequence_context_size: runtime.model_runtime_parameters.context_size.get(),
    }
    .into_generation_request_preparer();
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
    assert_eq!(
        generated_tokens_rx.try_recv(),
        Ok(GeneratedTokenResult::PromptTokenizedToNothing(
            "test-agent: the prompt tokenized to no tokens, so there is nothing to generate from"
                .to_owned()
        ))
    );
}
