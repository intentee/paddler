#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_tests::openai_chat_completions_client::OpenAIChatCompletionsClient;
use paddler_tests::start_in_process_cluster_with_qwen3::start_in_process_cluster_with_qwen3;
use reqwest::Client;
use serde_json::Value;
use serde_json::json;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn qwen3_openai_streaming_routes_reasoning_to_reasoning_content() -> Result<()> {
    let cluster = start_in_process_cluster_with_qwen3(1).await?;
    let openai_client = OpenAIChatCompletionsClient::new(
        Client::new(),
        &cluster.addresses.compat_openai_base_url()?,
    )?;

    let chunks = openai_client
        .post_streaming(&json!({
            "model": "qwen3-test",
            "messages": [{"role": "user", "content": "What is two plus two? Think step by step."}],
            "stream": true,
            "max_completion_tokens": 600
        }))
        .await?;

    let reasoning_chunks = chunks
        .iter()
        .filter(|chunk| {
            chunk
                .pointer("/choices/0/delta/reasoning_content")
                .and_then(Value::as_str)
                .is_some()
        })
        .count();

    assert!(
        reasoning_chunks > 0,
        "expected at least one delta.reasoning_content chunk; got {reasoning_chunks}"
    );

    cluster.shutdown().await?;

    Ok(())
}
