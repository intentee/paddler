#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::gbnf_literal::gbnf_literal;
use paddler_tests::get_weather_tool::get_weather_tool;
use paddler_tests::max_tokens_for_forced_text::max_tokens_for_forced_text;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use paddler_tests::weather_question_conversation::weather_question_conversation;

const FORCED_OUTPUT: &str = "<tool_call></tool_call>";

#[tokio::test(flavor = "multi_thread")]
async fn an_empty_tool_call_section_resolves_to_no_tool_calls() {
    let cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 1))
        .await
        .expect("the cluster must start");

    let token_results = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &weather_question_conversation(
                gbnf_literal(FORCED_OUTPUT),
                max_tokens_for_forced_text(FORCED_OUTPUT),
                vec![get_weather_tool()],
            ),
        )
        .await
        .expect("the inference request must be accepted")
        .into_token_results();

    assert_eq!(
        token_results
            .iter()
            .filter(|token_result| token_result.is_tool_call_parsed())
            .collect::<Vec<_>>(),
        vec![&GeneratedTokenResult::ToolCallParsed(Vec::new())]
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
