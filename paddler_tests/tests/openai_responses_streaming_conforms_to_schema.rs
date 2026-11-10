#![cfg(feature = "tests_that_use_llms")]

use paddler_openai_response_format_validator::openai_validator::OpenAIValidator;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use paddler_tests::streamed_greeting_responses_request::streamed_greeting_responses_request;

#[tokio::test(flavor = "multi_thread")]
async fn openai_responses_streaming_conforms_to_schema() {
    let validator = OpenAIValidator::new().expect("the OpenAI schemas must load");
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");
    let request = streamed_greeting_responses_request();

    validator
        .validate_responses_request(&request)
        .expect("the payload must conform to the OpenAI schema");

    let events = cluster
        .openai_responses_streaming(&request)
        .await
        .expect("the OpenAI response stream must succeed");

    assert!(!events.is_empty(), "expected at least one streaming event");

    for event in &events {
        validator
            .validate_responses_stream_event(event)
            .expect("the payload must conform to the OpenAI schema");
    }

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
