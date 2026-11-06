#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::raw_tool_call_tokens::RawToolCallTokens;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::gbnf_literal::gbnf_literal;
use paddler_tests::get_weather_tool::get_weather_tool;
use paddler_tests::max_tokens_for_forced_text::max_tokens_for_forced_text;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use paddler_tests::weather_question_conversation::weather_question_conversation;

const TOOL_CALL_FOLLOWED_BY_CONTENT: &str = "<tool_call>\n{\"name\": \"get_weather\", \"arguments\": {\"location\": \"Paris\"}}\n</tool_call>\nI asked for the weather in Paris.";

#[tokio::test(flavor = "multi_thread")]
async fn agent_reports_content_after_a_tool_call_as_an_unrecognized_tool_call_format() {
    let cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 1))
        .await
        .expect("the cluster must start");

    let token_results = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &weather_question_conversation(
                gbnf_literal(TOOL_CALL_FOLLOWED_BY_CONTENT),
                max_tokens_for_forced_text(TOOL_CALL_FOLLOWED_BY_CONTENT),
                vec![get_weather_tool()],
            ),
        )
        .await
        .expect("the inference request must be accepted")
        .into_token_results();

    assert_eq!(
        token_results
            .iter()
            .filter(|token_result| token_result.is_tool_call_parsed()
                || matches!(
                    token_result,
                    GeneratedTokenResult::UnrecognizedToolCallFormat(_)
                ))
            .collect::<Vec<_>>(),
        vec![&GeneratedTokenResult::UnrecognizedToolCallFormat(
            RawToolCallTokens {
                text: TOOL_CALL_FOLLOWED_BY_CONTENT.to_owned(),
                expected_format: "peg-native".to_owned(),
            }
        )]
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
