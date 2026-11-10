#![cfg(feature = "tests_that_use_llms")]

use serde_json::Value;
use serde_json::json;

use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn openai_chat_completion_non_streaming_returns_text() {
    let cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 2))
        .await
        .expect("the cluster must start");

    let body = cluster
        .openai_chat_completion_non_streaming(&json!({
            "model": "test",
            "messages": [{"role": "user", "content": "Say hello"}],
            "max_completion_tokens": 200,
            "stream": false,
        }))
        .await
        .expect("the OpenAI chat completion must succeed");

    assert_eq!(
        body.pointer("/object").and_then(Value::as_str),
        Some("chat.completion")
    );
    assert!(body.pointer("/choices").is_some_and(Value::is_array));
    assert!(
        body.pointer("/choices/0/message/content")
            .and_then(Value::as_str)
            .is_some_and(|content| !content.is_empty()),
        "response content should not be empty"
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
