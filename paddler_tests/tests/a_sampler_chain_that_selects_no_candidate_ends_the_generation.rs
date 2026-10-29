#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings::sampling::LlamaSampler;

use paddler_agent_text_generation::continuous_batch_terminal_outcome::ContinuousBatchTerminalOutcome;
use paddler_agent_text_generation::grammar_sampling::GrammarSampling;
use paddler_agent_text_generation::prepared_generation_request::PreparedGenerationRequest;
use paddler_agent_text_generation::token_sampling::TokenSampling;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::kev_checkpoint::KevCheckpoint;
use paddler_tests::llama_runtime::LlamaRuntime;

use crate::decode_prompt_for_sampling::decode_prompt_for_sampling;
use crate::ready_to_sample_request::ReadyToSampleRequest;

const PROMPT: &str = "Paddler balances";

#[test]
fn a_sampler_chain_that_selects_no_candidate_ends_the_generation() {
    let runtime = LlamaRuntime::load(
        &KevCheckpoint::Kev0_8b.target_path("model.gguf"),
        agent_inference_runtime_context(1),
    )
    .expect("the kev model must load");
    let mut llama_context = runtime
        .create_text_generation_llama_context(1)
        .expect("the context must be created");
    let batch_position = decode_prompt_for_sampling(&runtime, &mut llama_context, PROMPT)
        .expect("the prompt must be decoded for sampling");

    assert_eq!(
        ReadyToSampleRequest {
            batch_position,
            llama_context: &llama_context,
            prompt: PROMPT,
            runtime: &runtime,
        }
        .advance(|prepared_request| PreparedGenerationRequest {
            token_sampling: TokenSampling {
                chain: LlamaSampler::top_k(1).expect("the top-k sampler must initialize"),
                grammar: GrammarSampling::Unconstrained,
            },
            ..prepared_request
        }),
        ContinuousBatchTerminalOutcome::EmitToClient(
            GeneratedTokenResult::SamplingCandidatesExhausted(
                "all token candidates were eliminated during sampling".to_owned()
            )
        )
    );
}
