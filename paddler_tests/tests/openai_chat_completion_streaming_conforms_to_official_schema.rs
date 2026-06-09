#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_openai_response_format_validator::openai_validator::OpenAIValidator;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn openai_chat_completion_streaming_conforms_to_official_schema() -> Result<()> {
    let validator = OpenAIValidator::new()?;
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)]).await?;

    let request = json!({
        "model": "qwen3-test",
        "messages": [{"role": "user", "content": "Say hello."}],
        "max_completion_tokens": 200,
        "stream": true,
        "stream_options": {"include_usage": true}
    });

    validator.validate_chat_completion_request(&request)?;

    let chunks = cluster.openai_chat_completion_streaming(&request).await?;

    assert!(!chunks.is_empty(), "expected at least one streaming chunk");

    for chunk in &chunks {
        validator.validate_chat_completion_stream_chunk(chunk)?;
    }

    cluster.shutdown().await?;

    Ok(())
}
