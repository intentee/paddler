#![cfg(feature = "tests_that_use_llms")]

use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use paddler_tests::streamed_greeting_responses_request::streamed_greeting_responses_request;

#[tokio::test(flavor = "multi_thread")]
async fn openai_responses_stream_runs_from_created_to_completed_with_gapless_sequence_numbers() {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");

    let events = cluster
        .openai_responses_streaming(&streamed_greeting_responses_request())
        .await
        .expect("the OpenAI response stream must succeed");

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
