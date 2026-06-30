#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_cluster::agent_config::AgentConfig;
use paddler_tests::openai_chat_completions_client::OpenAIChatCompletionsClient;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn agent_openai_chat_completions_streaming_returns_chunks() -> Result<()> {
    let cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 2)).await?;

    let openai_base_url = cluster.balancer.addresses.compat_openai_base_url()?;
    let client = OpenAIChatCompletionsClient::new(&openai_base_url)?;

    let chunks = client
        .post_streaming(&json!({
            "model": "test",
            "messages": [{"role": "user", "content": "Say hello"}],
            "max_completion_tokens": 10,
        }))
        .await?;

    assert!(!chunks.is_empty(), "should have received streaming chunks");
    assert_eq!(chunks[0]["object"], "chat.completion.chunk");

    cluster.shutdown().await?;

    Ok(())
}
