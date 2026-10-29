#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings::error::LogitsError;

use paddler_agent_text_generation::continuous_batch_terminal_outcome::ContinuousBatchTerminalOutcome;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::kev_checkpoint::KevCheckpoint;
use paddler_tests::llama_runtime::LlamaRuntime;

use crate::ready_to_sample_request::ReadyToSampleRequest;

#[test]
fn a_token_the_sampler_cannot_read_ends_the_generation() {
    let runtime = LlamaRuntime::load(
        &KevCheckpoint::Kev0_8b.target_path("model.gguf"),
        agent_inference_runtime_context(1),
    )
    .expect("the kev model must load");
    let llama_context = runtime
        .create_text_generation_llama_context(1)
        .expect("the context must be created");

    assert_eq!(
        ReadyToSampleRequest {
            batch_position: 0,
            llama_context: &llama_context,
            prompt: "Paddler",
            runtime: &runtime,
        }
        .advance(|prepared_request| prepared_request),
        ContinuousBatchTerminalOutcome::EmitToClient(GeneratedTokenResult::SamplerError(
            TextGenerationError::TokenDataUnreadable(LogitsError::TokenNotInitialized(0))
                .to_string()
        ))
    );
}
