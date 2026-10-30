#![cfg(feature = "tests_that_use_llms")]

use serde_json::json;
use std::num::NonZeroU32;

use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::oversized_text::oversized_text;
use paddler_tests::start_cluster_with_qwen3_and_context_size::start_cluster_with_qwen3_and_context_size;

const SEQUENCE_CONTEXT_SIZE: NonZeroU32 = NonZeroU32::new(256).unwrap();

#[tokio::test(flavor = "multi_thread")]
async fn openai_chat_completion_streaming_reports_a_conversation_exceeding_the_context_as_an_error_chunk()
 {
    let cluster = start_cluster_with_qwen3_and_context_size(
        vec![AgentConfig::single(1)],
        SEQUENCE_CONTEXT_SIZE,
    )
    .await
    .expect("the cluster must start");

    let chunks = cluster
        .openai_chat_completion_streaming(&json!({
            "model": "test",
            "messages": [{"role": "user", "content": oversized_text()}],
            "stream": true,
        }))
        .await
        .expect("the OpenAI chat completion stream must be readable");

    assert_eq!(
        chunks
            .iter()
            .map(|chunk| chunk["error"]["type"].clone())
            .collect::<Vec<_>>(),
        vec![json!("invalid_request_error")]
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
