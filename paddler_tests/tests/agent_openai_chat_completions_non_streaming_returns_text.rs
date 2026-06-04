#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use serde_json::json;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn agent_openai_chat_completions_non_streaming_returns_text() -> Result<()> {
    let cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 2)).await?;

    let openai_url = cluster
        .balancer
        .addresses
        .compat_openai_base_url()?
        .join("v1/chat/completions")?;

    let response = reqwest::Client::new()
        .post(openai_url)
        .json(&json!({
            "model": "test",
            "messages": [{"role": "user", "content": "Say hello"}],
            "max_completion_tokens": 200,
            "stream": false,
        }))
        .send()
        .await
        .context("OpenAI compat request should succeed")?;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.context("response should be JSON")?;

    assert_eq!(body["object"], "chat.completion");
    assert!(body["choices"].is_array());
    assert!(
        !body["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .is_empty(),
        "response content should not be empty"
    );

    cluster.shutdown().await?;

    Ok(())
}
