#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use anyhow::Context as _;
use anyhow::Result;
use paddler_tests::start_subprocess_cluster_with_qwen3::start_subprocess_cluster_with_qwen3;
use serde_json::json;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn agent_openai_chat_completions_streaming_returns_chunks() -> Result<()> {
    let cluster = start_subprocess_cluster_with_qwen3(2, 1).await?;

    let openai_url = cluster
        .addresses
        .compat_openai_base_url()?
        .join("v1/chat/completions")?;

    let response = reqwest::Client::new()
        .post(openai_url)
        .json(&json!({
            "model": "test",
            "messages": [{"role": "user", "content": "Say hello"}],
            "max_completion_tokens": 10,
            "stream": true,
        }))
        .send()
        .await
        .context("OpenAI compat streaming request should succeed")?;

    assert_eq!(response.status(), 200);

    let body = response.text().await.context("should read response body")?;

    let chunks: Vec<serde_json::Value> = body
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let stripped = line.strip_prefix("data: ").unwrap_or(line);

            serde_json::from_str(stripped).context("each chunk should be valid JSON")
        })
        .collect::<Result<_>>()?;

    assert!(!chunks.is_empty(), "should have received streaming chunks");
    assert_eq!(chunks[0]["object"], "chat.completion.chunk");

    cluster.shutdown().await?;

    Ok(())
}
