#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_cluster::agent_config::AgentConfig;
use paddler_tests::cluster_openai_compat::ClusterOpenAiCompat;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn agent_openai_chat_completions_non_streaming_returns_text() -> Result<()> {
    let cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 2)).await?;

    let response = cluster
        .openai_chat_completion_non_streaming(&json!({
            "model": "test",
            "messages": [{"role": "user", "content": "Say hello"}],
            "max_completion_tokens": 200,
            "stream": false,
        }))
        .await?;

    assert_eq!(response["object"], "chat.completion");
    assert!(response["choices"].is_array());
    assert!(
        !response["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .is_empty(),
        "response content should not be empty"
    );

    cluster.shutdown().await?;

    Ok(())
}
