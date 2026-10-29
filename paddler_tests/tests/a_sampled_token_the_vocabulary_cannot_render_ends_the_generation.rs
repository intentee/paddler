#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings::error::TokenToStringError;
use llama_cpp_bindings::sampling::LlamaSampler;
use llama_cpp_bindings::token::LlamaToken;
use llama_cpp_bindings::token::logit_bias::LlamaLogitBias;

use paddler_agent_text_generation::continuous_batch_terminal_outcome::ContinuousBatchTerminalOutcome;
use paddler_agent_text_generation::grammar_sampling::GrammarSampling;
use paddler_agent_text_generation::prepared_generation_request::PreparedGenerationRequest;
use paddler_agent_text_generation::token_sampling::TokenSampling;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::llama_runtime::LlamaRuntime;
use paddler_tests::qwen3_0_6b_unused_padding_token::QWEN3_0_6B_UNUSED_PADDING_TOKEN;

use crate::decode_prompt_for_sampling::decode_prompt_for_sampling;
use crate::ready_to_sample_request::ReadyToSampleRequest;

const PROMPT: &str = "Paddler balances";

#[test]
fn a_sampled_token_the_vocabulary_cannot_render_ends_the_generation() {
    let runtime = LlamaRuntime::load(
        &qwen3_0_6b()
            .cached_path()
            .expect("the model must be in the Hugging Face cache"),
        agent_inference_runtime_context(1),
    )
    .expect("the model must load");
    let mut llama_context = runtime
        .create_text_generation_llama_context(1)
        .expect("the context must be created");
    let batch_position = decode_prompt_for_sampling(&runtime, &mut llama_context, PROMPT)
        .expect("the prompt must be decoded for sampling");
    let unrenderable_token = LlamaToken::new(
        i32::try_from(QWEN3_0_6B_UNUSED_PADDING_TOKEN).expect("the padding token must fit i32"),
    );

    assert_eq!(
        ReadyToSampleRequest {
            batch_position,
            llama_context: &llama_context,
            prompt: PROMPT,
            runtime: &runtime,
        }
        .advance(|prepared_request| PreparedGenerationRequest {
            token_sampling: TokenSampling {
                chain: LlamaSampler::chain_simple([
                    LlamaSampler::logit_bias(
                        runtime.loaded_llama_model.model.n_vocab(),
                        &[LlamaLogitBias::new(unrenderable_token, f32::INFINITY)],
                    )
                    .expect("the logit bias sampler must initialize"),
                    LlamaSampler::greedy().expect("the greedy sampler must initialize"),
                ])
                .expect("the sampler chain must initialize"),
                grammar: GrammarSampling::Unconstrained,
            },
            ..prepared_request
        }),
        ContinuousBatchTerminalOutcome::EmitToClient(GeneratedTokenResult::DetokenizationFailed(
            TokenToStringError::UnknownTokenType.to_string()
        ))
    );
}
