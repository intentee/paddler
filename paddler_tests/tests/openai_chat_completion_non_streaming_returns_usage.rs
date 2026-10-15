#![cfg(feature = "tests_that_use_llms")]

use serde_json::Value;
use serde_json::json;

use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn openai_chat_completion_non_streaming_returns_usage() {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");

    let response = cluster
        .openai_chat_completion_non_streaming(&json!({
            "model": "qwen3-test",
            "messages": [{"role": "user", "content": "Say hi briefly."}],
            "max_completion_tokens": 600
        }))
        .await
        .expect("the OpenAI chat completion must succeed");

    let usage = response
        .get("usage")
        .unwrap_or_else(|| panic!("non-streaming response missing usage: {response}"));

    let prompt_tokens = usage
        .get("prompt_tokens")
        .and_then(Value::as_u64)
        .expect("usage.prompt_tokens missing");
    let completion_tokens = usage
        .get("completion_tokens")
        .and_then(Value::as_u64)
        .expect("usage.completion_tokens missing");
    let total_tokens = usage
        .get("total_tokens")
        .and_then(Value::as_u64)
        .expect("usage.total_tokens missing");

    assert!(prompt_tokens > 0);
    assert!(completion_tokens > 0);
    assert_eq!(total_tokens, prompt_tokens + completion_tokens);

    let content = response
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .expect("non-streaming response missing message content");

    assert!(
        !content.is_empty(),
        "non-streaming content must not be empty"
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
