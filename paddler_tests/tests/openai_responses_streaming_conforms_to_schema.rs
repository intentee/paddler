#![cfg(feature = "tests_that_use_llms")]

use serde_json::json;

use paddler_openai_response_format_validator::openai_validator::OpenAIValidator;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn openai_responses_streaming_conforms_to_schema() {
    let validator = OpenAIValidator::new().expect("the OpenAI schemas must load");
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");

    let request = json!({
        "model": "qwen3-test",
        "input": "Say hello.",
        "max_output_tokens": 200,
        "stream": true
    });

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

    assert_eq!(
        events.first().and_then(|event| event["type"].as_str()),
        Some("response.created"),
        "the responses stream must begin with response.created"
    );
    assert_eq!(
        events.last().and_then(|event| event["type"].as_str()),
        Some("response.completed"),
        "the responses stream must terminate with response.completed"
    );

    let sequence_numbers: Vec<u64> = events
        .iter()
        .filter_map(|event| event["sequence_number"].as_u64())
        .collect();

    assert_eq!(
        sequence_numbers,
        (0..sequence_numbers.len() as u64).collect::<Vec<_>>(),
        "sequence numbers must be a gapless run starting at 0"
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
