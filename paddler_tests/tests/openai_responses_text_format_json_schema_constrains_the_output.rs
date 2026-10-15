#![cfg(feature = "tests_that_use_llms")]

use serde_json::Value;
use serde_json::from_str;
use serde_json::json;

use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::responses_message_text::responses_message_text;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn openai_responses_text_format_json_schema_constrains_the_output() {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");

    let response = cluster
        .openai_responses_non_streaming(&json!({
            "model": "qwen3-test",
            "input": "What is 2+2?",
            "max_output_tokens": 50,
            "reasoning": { "effort": "none" },
            "text": {
                "format": {
                    "type": "json_schema",
                    "name": "sum",
                    "schema": {
                        "type": "object",
                        "properties": { "answer": { "type": "integer" } },
                        "required": ["answer"],
                        "additionalProperties": false
                    }
                }
            }
        }))
        .await
        .expect("the OpenAI response must succeed");

    let answer: Value = from_str(
        responses_message_text(&response).expect("the Responses output must carry message text"),
    )
    .expect("the output must be JSON");

    assert_eq!(
        answer
            .as_object()
            .map(|answer_fields| answer_fields.keys().map(String::as_str).collect::<Vec<_>>()),
        Some(vec!["answer"])
    );
    assert!(answer["answer"].is_i64(), "{answer}");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
