#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_tests::openai_chat_completions_client::OpenAIChatCompletionsClient;
use paddler_tests::start_in_process_cluster_with_qwen3::start_in_process_cluster_with_qwen3;
use reqwest::Client;
use serde_json::Value;
use serde_json::json;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn qwen3_openai_streaming_emits_usage_when_requested() -> Result<()> {
    let cluster = start_in_process_cluster_with_qwen3(1).await?;
    let openai_client = OpenAIChatCompletionsClient::new(
        Client::new(),
        &cluster.addresses.compat_openai_base_url()?,
    )?;

    let chunks = openai_client
        .post_streaming(&json!({
            "model": "qwen3-test",
            "messages": [{"role": "user", "content": "Say hi briefly."}],
            "stream": true,
            "stream_options": {"include_usage": true},
            "max_completion_tokens": 80
        }))
        .await?;

    let last_chunk = chunks
        .last()
        .ok_or_else(|| anyhow::anyhow!("no chunks received from streaming endpoint"))?;

    let usage = last_chunk
        .get("usage")
        .ok_or_else(|| anyhow::anyhow!("trailing chunk lacks usage field: {last_chunk}"))?;

    let prompt_tokens = usage
        .get("prompt_tokens")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("usage.prompt_tokens missing or not u64"))?;
    let completion_tokens = usage
        .get("completion_tokens")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("usage.completion_tokens missing or not u64"))?;
    let total_tokens = usage
        .get("total_tokens")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("usage.total_tokens missing or not u64"))?;

    assert!(prompt_tokens > 0);
    assert!(completion_tokens > 0);
    assert_eq!(total_tokens, prompt_tokens + completion_tokens);

    let trailing_choices = last_chunk
        .get("choices")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("trailing chunk lacks choices array"))?;

    assert!(
        trailing_choices.is_empty(),
        "OpenAI usage chunk must have empty choices array, got: {trailing_choices:?}"
    );

    cluster.shutdown().await?;

    Ok(())
}
