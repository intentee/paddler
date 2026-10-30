#![cfg(feature = "tests_that_use_llms")]

use serde_json::Value;
use serde_json::json;

use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn openai_chat_completion_with_reasoning_effort_none_generates_no_reasoning_tokens() {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");

    let response = cluster
        .openai_chat_completion_non_streaming(&json!({
            "model": "qwen3-test",
            "messages": [{"role": "user", "content": "What is two plus two?"}],
            "max_completion_tokens": 200,
            "reasoning_effort": "none"
        }))
        .await
        .expect("the OpenAI chat completion must succeed");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");

    assert_eq!(
        response
            .pointer("/usage/completion_tokens_details/reasoning_tokens")
            .and_then(Value::as_u64),
        Some(0)
    );
}
