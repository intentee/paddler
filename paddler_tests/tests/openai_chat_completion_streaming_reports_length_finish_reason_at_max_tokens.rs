#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_openai_response_format_validator::openai_validator::OpenAIValidator;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use serde_json::Value;
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn openai_chat_completion_streaming_reports_length_finish_reason_at_max_tokens() -> Result<()>
{
    let validator = OpenAIValidator::new()?;
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)]).await?;

    let chunks = cluster
        .openai_chat_completion_streaming(&json!({
            "model": "qwen3-test",
            "messages": [{"role": "user", "content": "Count from one to one hundred."}],
            "stream": true,
            "max_completion_tokens": 5
        }))
        .await?;

    for chunk in &chunks {
        validator.validate_chat_completion_stream_chunk(chunk)?;
    }

    let finish_reasons: Vec<&Value> = chunks
        .iter()
        .map(|chunk| &chunk["choices"][0]["finish_reason"])
        .filter(|finish_reason| !finish_reason.is_null())
        .collect();

    assert_eq!(finish_reasons, vec![&json!("length")]);

    cluster.shutdown().await?;

    Ok(())
}
