#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use anyhow::Result;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::gbnf_literal::gbnf_literal;
use paddler_tests::get_weather_tool::get_weather_tool;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use paddler_tests::weather_question_conversation::weather_question_conversation;
use tokio_util::sync::CancellationToken;

const UNFINISHED_TOOL_CALL_JSON: &str = r#"{"name": "get_weather""#;

#[tokio::test(flavor = "multi_thread")]
async fn agent_streams_tokens_held_by_the_bare_json_probe_at_end_of_generation() -> Result<()> {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)]).await?;

    let collected = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &weather_question_conversation(
                gbnf_literal(UNFINISHED_TOOL_CALL_JSON),
                NonZeroU32::new(100).unwrap(),
                vec![get_weather_tool()],
            ),
        )
        .await?;

    assert_eq!(collected.text, UNFINISHED_TOOL_CALL_JSON);
    assert_eq!(collected.summary()?.usage.tool_call_tokens, 0);

    cluster.shutdown().await?;

    Ok(())
}
