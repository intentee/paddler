#![cfg(feature = "tests_that_use_llms")]

use serde_json::Value;
use serde_json::json;

use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::responses_message_text::responses_message_text;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn openai_responses_non_streaming_returns_text_and_usage() {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");

    let response = cluster
        .openai_responses_non_streaming(&json!({
            "model": "qwen3-test",
            "input": "Say hi briefly.",
            "max_output_tokens": 600
        }))
        .await
        .expect("the OpenAI response must succeed");

    assert_eq!(
        response.get("status").and_then(Value::as_str),
        Some("completed"),
        "responses status must be completed: {response}"
    );

    let usage = response
        .get("usage")
        .expect("the Responses response must report its usage");

    let input_tokens = usage
        .get("input_tokens")
        .and_then(Value::as_u64)
        .expect("usage.input_tokens missing");
    let output_tokens = usage
        .get("output_tokens")
        .and_then(Value::as_u64)
        .expect("usage.output_tokens missing");
    let total_tokens = usage
        .get("total_tokens")
        .and_then(Value::as_u64)
        .expect("usage.total_tokens missing");

    assert!(input_tokens > 0);
    assert!(output_tokens > 0);
    assert_eq!(total_tokens, input_tokens + output_tokens);

    let message_text =
        responses_message_text(&response).expect("the Responses output must carry message text");

    assert!(
        !message_text.is_empty(),
        "responses message text must not be empty"
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
