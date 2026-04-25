#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use anyhow::Context as _;
use anyhow::Result;
use paddler_tests::start_subprocess_cluster_with_qwen3::start_subprocess_cluster_with_qwen3;
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn agent_raw_prompt_without_grammar_field_succeeds() -> Result<()> {
    let cluster = start_subprocess_cluster_with_qwen3(2, 1).await?;

    let inference_url = cluster
        .addresses
        .inference_base_url()?
        .join("api/v1/continue_from_raw_prompt")?;

    let response = reqwest::Client::new()
        .post(inference_url)
        .json(&json!({
            "max_tokens": 10,
            "raw_prompt": "Hello",
        }))
        .send()
        .await
        .context("HTTP request must succeed")?;

    assert!(
        response.status().is_success(),
        "request without grammar field must succeed (backwards compatible); status: {}",
        response.status()
    );

    cluster.shutdown().await?;

    Ok(())
}
