#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::openai_chat_completions_client::OpenAIChatCompletionsClient;
use paddler_tests::start_in_process_cluster_with_qwen3::start_in_process_cluster_with_qwen3;
use reqwest::Client;
use serde_json::json;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn qwen3_openai_streaming_omits_usage_when_not_requested() -> Result<()> {
    let cluster = start_in_process_cluster_with_qwen3(AgentConfig::single(1)).await?;
    let openai_client = OpenAIChatCompletionsClient::new(
        Client::new(),
        &cluster.addresses.compat_openai_base_url()?,
    )?;

    let chunks = openai_client
        .post_streaming(&json!({
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
