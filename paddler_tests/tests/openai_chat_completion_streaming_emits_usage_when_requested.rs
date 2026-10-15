#![cfg(feature = "tests_that_use_llms")]

use serde_json::Value;
use serde_json::json;

use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn openai_chat_completion_streaming_emits_usage_when_requested() {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");

    let chunks = cluster
        .openai_chat_completion_streaming(&json!({
            "model": "qwen3-test",
            "messages": [{"role": "user", "content": "Say hi briefly."}],
            "stream": true,
            "stream_options": {"include_usage": true},
            "max_completion_tokens": 80
        }))
        .await
        .expect("the OpenAI chat completion stream must succeed");

    let last_chunk = chunks
        .last()
        .expect("no chunks received from streaming endpoint");

    let usage = last_chunk
        .get("usage")
        .unwrap_or_else(|| panic!("trailing chunk lacks usage field: {last_chunk}"));

    let prompt_tokens = usage
        .get("prompt_tokens")
        .and_then(Value::as_u64)
        .expect("usage.prompt_tokens missing or not u64");
    let completion_tokens = usage
        .get("completion_tokens")
        .and_then(Value::as_u64)
        .expect("usage.completion_tokens missing or not u64");
    let total_tokens = usage
        .get("total_tokens")
        .and_then(Value::as_u64)
        .expect("usage.total_tokens missing or not u64");

    assert!(prompt_tokens > 0);
    assert!(completion_tokens > 0);
    assert_eq!(total_tokens, prompt_tokens + completion_tokens);

    let trailing_choices = last_chunk
        .get("choices")
        .and_then(Value::as_array)
        .expect("trailing chunk lacks choices array");

    assert!(
        trailing_choices.is_empty(),
        "OpenAI usage chunk must have empty choices array, got: {trailing_choices:?}"
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
