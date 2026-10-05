#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::gbnf_literal::gbnf_literal;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use paddler_tests::weather_question_conversation::weather_question_conversation;
use paddler_tests::weather_tool_call_json::WEATHER_TOOL_CALL_JSON;

#[tokio::test(flavor = "multi_thread")]
async fn agent_streams_json_grammar_output_as_content_when_no_tools_are_declared() {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");

    let collected = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &weather_question_conversation(
                gbnf_literal(WEATHER_TOOL_CALL_JSON),
                NonZeroU32::new(100).unwrap(),
                vec![],
            ),
        )
        .await
        .expect("the inference request must be accepted");

    assert_eq!(collected.text, WEATHER_TOOL_CALL_JSON);
    assert!(
        !collected
            .token_results
            .iter()
            .any(|token_result_with_producer| matches!(
                token_result_with_producer.token_result,
                GeneratedTokenResult::ToolCallToken(_)
            )),
        "JSON output must not be classified as a tool call when no tools are declared: {:?}",
        collected.token_results
    );
    assert_eq!(
        collected
            .summary()
            .expect("the generation must finish with a summary")
            .usage
            .tool_call_tokens,
        0
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
