#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::openai_chat_completions_client::OpenAIChatCompletionsClient;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use reqwest::Client;
use serde_json::Value;
use serde_json::json;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn qwen3_openai_non_streaming_returns_usage() -> Result<()> {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)]).await?;
    let openai_client = OpenAIChatCompletionsClient::new(
        Client::new(),
        &cluster.addresses.compat_openai_base_url()?,
    )?;

    let response = openai_client
        .post_non_streaming(&json!({
            "model": "qwen3-test",
            "messages": [{"role": "user", "content": "Say hi briefly."}],
            "max_completion_tokens": 600
        }))
        .await?;

    let usage = response
        .get("usage")
        .ok_or_else(|| anyhow::anyhow!("non-streaming response missing usage: {response}"))?;

    let prompt_tokens = usage
        .get("prompt_tokens")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("usage.prompt_tokens missing"))?;
    let completion_tokens = usage
        .get("completion_tokens")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("usage.completion_tokens missing"))?;
    let total_tokens = usage
        .get("total_tokens")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("usage.total_tokens missing"))?;

    assert!(prompt_tokens > 0);
    assert!(completion_tokens > 0);
    assert_eq!(total_tokens, prompt_tokens + completion_tokens);

    let content = response
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("non-streaming response missing message content"))?;

    assert!(
        !content.is_empty(),
        "non-streaming content must not be empty"
    );

    cluster.shutdown().await?;

    Ok(())
}
