#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::gbnf_literal::gbnf_literal;
use paddler_tests::get_weather_tool::get_weather_tool;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use paddler_tests::weather_question_conversation::weather_question_conversation;
use paddler_tests::weather_tool_call_json::WEATHER_TOOL_CALL_JSON;

const MAX_TOKENS: NonZeroU32 = NonZeroU32::new(3).unwrap();

#[tokio::test(flavor = "multi_thread")]
async fn agent_stops_at_max_tokens_while_the_bare_json_probe_holds_tokens() {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");

    let collected = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &weather_question_conversation(
                gbnf_literal(WEATHER_TOOL_CALL_JSON),
                MAX_TOKENS,
                vec![get_weather_tool()],
            ),
        )
        .await
        .expect("the inference request must be accepted");

    let streamed_token_count = collected
        .token_results
        .iter()
        .filter(|token_result_with_producer| token_result_with_producer.token_result.is_token())
        .count();

    assert_eq!(streamed_token_count, 3, "{:?}", collected.token_results);
    assert_eq!(
        collected
            .summary()
            .expect("the generation must finish with a summary")
            .usage
            .completion_tokens(),
        3
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
