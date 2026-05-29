#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use serde_json::json;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn agent_conversation_without_grammar_field_succeeds() -> Result<()> {
    let cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 2)).await?;

    let inference_url = cluster
        .balancer
        .addresses
        .inference_base_url()?
        .join("api/v1/continue_from_conversation_history")?;

    let response = reqwest::Client::new()
        .post(inference_url)
        .json(&json!({
            "add_generation_prompt": true,
            "conversation_history": [
                {"content": "Hello", "role": "user"}
            ],
            "enable_thinking": false,
            "max_tokens": 10,
            "tools": [],
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
