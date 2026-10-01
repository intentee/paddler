#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::token_result_with_producer::TokenResultWithProducer;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_stops_generation_when_stop_sender_dropped() {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(2)])
        .await
        .expect("the cluster must start");

    let mut first_stream = cluster
        .continue_from_raw_prompt_stream(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::new(500).unwrap(),
                raw_prompt: "Write a long essay about photosynthesis".to_owned(),
            },
        )
        .await
        .expect("the inference request must be accepted");

    let _first_token = first_stream
        .next()
        .await
        .expect("first stream must yield at least one message");

    drop(first_stream);

    let second_collected = cluster
        .continue_from_raw_prompt(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::new(10).unwrap(),
                raw_prompt: "Hello".to_owned(),
            },
        )
        .await
        .expect("the inference request must be accepted");

    assert!(matches!(
        second_collected.token_results.last(),
        Some(TokenResultWithProducer {
            token_result: GeneratedTokenResult::Done(_),
            ..
        })
    ));

    let second_token_count = second_collected
        .token_results
        .iter()
        .filter(|result| result.token_result.is_token())
        .count();

    assert!(
        second_token_count > 0,
        "second sequential request must succeed after first stream is dropped"
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
