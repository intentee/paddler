#![cfg(feature = "tests_that_use_llms")]

use serde_json::json;

use paddler_openai_response_format_validator::openai_validator::OpenAIValidator;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn openai_responses_non_streaming_reports_incomplete_status_at_max_output_tokens() {
    let validator = OpenAIValidator::new().expect("the OpenAI schemas must load");
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");

    let response = cluster
        .openai_responses_non_streaming(&json!({
            "model": "qwen3-test",
            "input": "Count from one to one hundred.",
            "max_output_tokens": 5
        }))
        .await
        .expect("the OpenAI response must succeed");

    validator
        .validate_responses_response(&response)
        .expect("the payload must conform to the OpenAI schema");

    assert_eq!(response["status"], "incomplete");
    assert_eq!(
        response["incomplete_details"],
        json!({"reason": "max_output_tokens"})
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
