#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn qwen3_openai_streaming_omits_usage_when_not_requested() -> Result<()> {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)]).await?;

    let chunks = cluster
        .openai_chat_completion_streaming(&json!({
            "model": "qwen3-test",
            "messages": [{"role": "user", "content": "Say hi briefly."}],
            "stream": true,
            "max_completion_tokens": 50
        }))
        .await?;

    assert!(!chunks.is_empty(), "expected at least one chunk");

    let chunks_with_usage = chunks
        .iter()
        .filter(|chunk| chunk.get("usage").is_some())
        .count();

    assert_eq!(
        chunks_with_usage, 0,
        "expected no usage chunks when stream_options.include_usage is absent, got {chunks_with_usage}"
    );

    cluster.shutdown().await?;

    Ok(())
}
