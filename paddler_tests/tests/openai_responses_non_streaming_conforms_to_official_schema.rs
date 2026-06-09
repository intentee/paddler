#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_openai_response_format_validator::openai_validator::OpenAIValidator;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn openai_responses_non_streaming_conforms_to_official_schema() -> Result<()> {
    let validator = OpenAIValidator::new()?;
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)]).await?;

    let request = json!({
        "model": "qwen3-test",
        "input": "Say hello.",
        "max_output_tokens": 200,
        "stream": false
    });

    validator.validate_responses_request(&request)?;

    let response = cluster.openai_responses_non_streaming(&request).await?;

    validator.validate_responses_response(&response)?;

    cluster.shutdown().await?;

    Ok(())
}
