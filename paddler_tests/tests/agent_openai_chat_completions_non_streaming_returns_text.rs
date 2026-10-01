#![cfg(feature = "tests_that_use_llms")]

use serde_json::json;

use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn agent_openai_chat_completions_non_streaming_returns_text() {
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

    assert_eq!(body["object"], "chat.completion");
    assert!(body["choices"].is_array());
    assert!(
        !body["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .is_empty(),
        "response content should not be empty"
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
