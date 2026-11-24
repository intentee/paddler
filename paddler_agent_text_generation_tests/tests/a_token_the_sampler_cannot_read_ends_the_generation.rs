#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_text_generation::continuous_batch_terminal_outcome::ContinuousBatchTerminalOutcome;
use paddler_cluster_harness::model_card::kev_0_8b::kev_0_8b;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::llama_runtime::LlamaRuntime;
use paddler_tests::prompt_sampling_positions::PromptSamplingPositions;
use paddler_tests::ready_to_sample_request::ReadyToSampleRequest;

#[test]
fn a_token_the_sampler_cannot_read_ends_the_generation() {
    let runtime = LlamaRuntime::load(
        &kev_0_8b()
            .cached_path()
            .expect("the kev model must be in the Hugging Face cache"),
        agent_inference_runtime_context(1),
    )
    .expect("the kev model must load");
    let llama_context = runtime
        .create_text_generation_llama_context(1)
        .expect("the context must be created");

    assert!(matches!(
        ReadyToSampleRequest {
            llama_context: &llama_context,
            prompt: "Paddler",
            prompt_sampling_positions: PromptSamplingPositions {
                batch_position: 0,
                next_token_position: 1,
            },
            runtime: &runtime,
        }
        .advance(|prepared_request| prepared_request)
        .expect("the request must be ready to sample"),
        ContinuousBatchTerminalOutcome::EmitToClient(GeneratedTokenResult::SamplerError(_))
    ));
}
