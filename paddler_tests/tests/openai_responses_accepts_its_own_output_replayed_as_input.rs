#![cfg(feature = "tests_that_use_llms")]

use serde_json::Value;
use serde_json::json;

use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn openai_responses_accepts_its_own_output_replayed_as_input() {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");
    let first_question = json!({ "type": "message", "role": "user", "content": "Say hi briefly." });

    let first_response = cluster
        .openai_responses_non_streaming(&json!({
            "model": "qwen3-test",
            "input": [first_question],
            "max_output_tokens": 600
        }))
        .await
        .expect("the first OpenAI response must succeed");
    let first_output = first_response
        .get("output")
        .and_then(Value::as_array)
        .expect("the first response must carry its output items");

    assert_eq!(
        first_output
            .iter()
            .map(|output_item| output_item.get("type").and_then(Value::as_str))
            .collect::<Vec<_>>(),
        vec![Some("reasoning"), Some("message")],
        "{first_response}"
    );

    let mut replayed_input = vec![first_question];

    replayed_input.extend(first_output.iter().cloned());
    replayed_input
        .push(json!({ "type": "message", "role": "user", "content": "Now say bye briefly." }));

    let second_response = cluster
        .openai_responses_non_streaming(&json!({
            "model": "qwen3-test",
            "input": replayed_input,
            "max_output_tokens": 600
        }))
        .await
        .expect("the response replaying the first output must succeed");

    assert_eq!(
        second_response.get("status").and_then(Value::as_str),
        Some("completed"),
        "{second_response}"
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
