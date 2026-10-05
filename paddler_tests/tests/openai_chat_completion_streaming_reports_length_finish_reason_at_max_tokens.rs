#![cfg(feature = "tests_that_use_llms")]

use serde_json::Value;
use serde_json::json;

use paddler_openai_response_format_validator::openai_validator::OpenAIValidator;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn openai_chat_completion_streaming_reports_length_finish_reason_at_max_tokens() {
    let validator = OpenAIValidator::new().expect("the OpenAI schemas must load");
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");

    let chunks = cluster
        .openai_chat_completion_streaming(&json!({
            "model": "qwen3-test",
            "messages": [{"role": "user", "content": "Count from one to one hundred."}],
            "stream": true,
            "max_completion_tokens": 5
        }))
        .await
        .expect("the OpenAI chat completion stream must succeed");

    for chunk in &chunks {
        validator
            .validate_chat_completion_stream_chunk(chunk)
            .expect("the payload must conform to the OpenAI schema");
    }

    let finish_reasons: Vec<&Value> = chunks
        .iter()
        .map(|chunk| &chunk["choices"][0]["finish_reason"])
        .filter(|finish_reason| !finish_reason.is_null())
        .collect();

    assert_eq!(finish_reasons, vec![&json!("length")]);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
