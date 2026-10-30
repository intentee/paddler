#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use llama_cpp_bindings::GbnfValidationError;
use llama_cpp_bindings::error::GrammarError;
use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::rejection_description::rejection_description;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::grammar_constraint::GrammarConstraint;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use paddler_tests::weather_question_conversation::weather_question_conversation;

#[tokio::test(flavor = "multi_thread")]
async fn agent_reports_grammar_syntax_error_for_malformed_gbnf() {
    let cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 2))
        .await
        .expect("the cluster must start");

    let malformed_grammar = GrammarConstraint::Gbnf {
        grammar: r#"root ::= "unterminated"#.to_owned(),
        root: "root".to_owned(),
    };
    let raw_prompt_results = cluster
        .continue_from_raw_prompt(CancellationToken::new(), &ContinueFromRawPromptParams {
            grammar: Some(malformed_grammar.clone()),
            max_tokens: NonZeroU32::new(10).unwrap(),
            raw_prompt:
                "<|im_start|>user\nSay hi.<|im_end|>\n<|im_start|>assistant\n<think>\n\n</think>\n\n"
                    .to_owned(),
        })
        .await
        .expect("the raw prompt request must be accepted")
        .into_token_results();
    let conversation_results = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &weather_question_conversation(malformed_grammar, NonZeroU32::new(10).unwrap(), vec![]),
        )
        .await
        .expect("the conversation request must be accepted")
        .into_token_results();

    for token_results in [raw_prompt_results, conversation_results] {
        assert_eq!(
            token_results,
            vec![GeneratedTokenResult::GrammarSyntaxError(
                rejection_description(
                    Some("test-agent-0"),
                    &TextGenerationError::GrammarInvalid(GrammarError::GrammarRejected(
                        GbnfValidationError::SyntaxError
                    ))
                )
            )]
        );
    }

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
