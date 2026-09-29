#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_openai_response_format_validator::openai_validator::OpenAIValidator;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn openai_responses_non_streaming_reports_incomplete_status_at_max_output_tokens()
-> Result<()> {
    let validator = OpenAIValidator::new()?;
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)]).await?;

    let response = cluster
        .openai_responses_non_streaming(&json!({
            "model": "qwen3-test",
            "input": "Count from one to one hundred.",
            "max_output_tokens": 5
        }))
        .await?;

    validator.validate_responses_response(&response)?;

    assert_eq!(response["status"], "incomplete");
    assert_eq!(
        response["incomplete_details"],
        json!({"reason": "max_output_tokens"})
    );

    cluster.shutdown().await?;

    Ok(())
}
