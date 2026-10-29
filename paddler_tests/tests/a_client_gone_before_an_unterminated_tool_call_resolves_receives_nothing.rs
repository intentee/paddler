#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings::sampling::LlamaSampler;
use llama_cpp_bindings::token::logit_bias::LlamaLogitBias;
use llama_cpp_bindings::token_piece::TokenPiece;
use tokio::sync::mpsc::unbounded_channel;

use paddler_agent_text_generation::continuous_batch_terminal_outcome::ContinuousBatchTerminalOutcome;
use paddler_agent_text_generation::grammar_sampling::GrammarSampling;
use paddler_agent_text_generation::prepared_generation_request::PreparedGenerationRequest;
use paddler_agent_text_generation::token_sampling::TokenSampling;
use paddler_agent_text_generation::tool_call_handling::ToolCallHandling;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::llama_runtime::LlamaRuntime;

use crate::decode_prompt_for_sampling::decode_prompt_for_sampling;
use crate::ready_to_sample_request::ReadyToSampleRequest;
use crate::tool_call_pipeline_for_tools::tool_call_pipeline_for_tools;

const PROMPT: &str = "Paddler balances";

#[test]
fn a_client_gone_before_an_unterminated_tool_call_resolves_receives_nothing() {
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
    let batch_position = decode_prompt_for_sampling(&runtime, &mut llama_context, PROMPT);
    let model = &runtime.loaded_llama_model.model;
    let end_of_generation = model
        .token_eos()
        .expect("the model must have an end-of-generation token");
    let mut unterminated_tool_call = tool_call_pipeline_for_tools(model, &[]);
    let (gone_client_tx, gone_client_rx) = unbounded_channel();

    unterminated_tool_call.feed(&TokenPiece::Marker("<tool_call>".to_owned()));
    drop(gone_client_rx);

    assert_eq!(
        ReadyToSampleRequest {
            batch_position,
            llama_context: &llama_context,
            prompt: PROMPT,
            runtime: &runtime,
        }
        .advance(|prepared_request| PreparedGenerationRequest {
            generated_tokens_tx: gone_client_tx,
            token_sampling: TokenSampling {
                chain: LlamaSampler::chain_simple([
                    LlamaSampler::logit_bias(
                        model.n_vocab(),
                        &[LlamaLogitBias::new(end_of_generation, f32::INFINITY)],
                    )
                    .expect("the logit bias sampler must initialize"),
                    LlamaSampler::greedy().expect("the greedy sampler must initialize"),
                ])
                .expect("the sampler chain must initialize"),
                grammar: GrammarSampling::Unconstrained,
            },
            tool_call_handling: ToolCallHandling::Parsed(unterminated_tool_call),
            ..prepared_request
        }),
        ContinuousBatchTerminalOutcome::EmitNothing
    );
}
