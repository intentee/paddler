#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings::sampling::LlamaSampler;

use paddler_agent_text_generation::continuous_batch_terminal_outcome::ContinuousBatchTerminalOutcome;
use paddler_agent_text_generation::grammar_sampling::GrammarSampling;
use paddler_agent_text_generation::prepared_generation_request::PreparedGenerationRequest;
use paddler_agent_text_generation::token_sampling::TokenSampling;
use paddler_messaging::generated_token_result::GeneratedTokenResult;

use crate::grammar_refusing_the_greedy_selection::GrammarRefusingTheGreedySelection;
use crate::ready_to_sample_request::ReadyToSampleRequest;

#[test]
fn a_grammar_refusing_the_token_that_triggers_it_ends_the_generation_as_rejected_model_output() {
    let refusal = GrammarRefusingTheGreedySelection::load();
    let mut llama_context = refusal
        .runtime
        .create_text_generation_llama_context(1)
        .expect("the context must be created");
    let batch_position = refusal.decode_the_prompt(&mut llama_context);

    assert_eq!(
        ReadyToSampleRequest {
            batch_position,
            llama_context: &llama_context,
            prompt: GrammarRefusingTheGreedySelection::PROMPT,
            runtime: &refusal.runtime,
        }
        .advance(|prepared_request| PreparedGenerationRequest {
            token_sampling: TokenSampling {
                chain: LlamaSampler::greedy().expect("the greedy sampler must initialize"),
                grammar: GrammarSampling::Constrained(
                    refusal.grammar_awaiting_the_greedy_selection()
                ),
            },
            ..prepared_request
        }),
        ContinuousBatchTerminalOutcome::EmitToClient(
            GeneratedTokenResult::GrammarRejectedModelOutput(
                GrammarRefusingTheGreedySelection::refusal().to_string()
            )
        )
    );
}
