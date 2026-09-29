#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use anyhow::anyhow;
use paddler_openai_response_format_validator::openai_validator::OpenAIValidator;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn openai_responses_streaming_reports_incomplete_status_at_max_output_tokens() -> Result<()> {
    let validator = OpenAIValidator::new()?;
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)]).await?;

    let events = cluster
        .openai_responses_streaming(&json!({
            "model": "qwen3-test",
            "input": "Count from one to one hundred.",
            "max_output_tokens": 5,
            "stream": true
        }))
        .await?;

    for event in &events {
        validator.validate_responses_stream_event(event)?;
    }

    let terminal_event = events
        .last()
        .ok_or_else(|| anyhow!("the responses stream sent no events"))?;

    assert_eq!(terminal_event["type"], "response.incomplete");
    assert_eq!(terminal_event["response"]["status"], "incomplete");
    assert_eq!(
        terminal_event["response"]["incomplete_details"],
        json!({"reason": "max_output_tokens"})
    );

    cluster.shutdown().await?;

    Ok(())
}
