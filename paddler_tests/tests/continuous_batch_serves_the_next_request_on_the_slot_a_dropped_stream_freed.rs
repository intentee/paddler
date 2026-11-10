#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::unending_generation::unending_generation;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_serves_the_next_request_on_the_slot_a_dropped_stream_freed() {
    let mut cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");

    let agent_id = cluster
        .agent_ids
        .first()
        .expect("cluster must have one registered agent")
        .clone();

    let mut unending_stream = cluster
        .continue_from_raw_prompt_stream(CancellationToken::new(), &unending_generation())
        .await
        .expect("the inference request must be accepted");

    let _first_message = unending_stream
        .next()
        .await
        .expect("the unending stream must yield at least one message");

    cluster
        .wait_for_slots_processing(&agent_id, 1)
        .await
        .expect("the unending generation must occupy the only slot");

    drop(unending_stream);

    cluster
        .wait_for_slots_processing(&agent_id, 0)
        .await
        .expect("dropping the unending stream must free the only slot");

    let next_collected = cluster
        .continue_from_raw_prompt(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::new(10).unwrap(),
                raw_prompt: "Hello".to_owned(),
            },
        )
        .await
        .expect("the next inference request must be accepted");

    next_collected
        .summary()
        .expect("the next generation must finish with a summary");

    assert!(
        next_collected
            .token_results
            .iter()
            .any(|token_result| token_result.token_result.is_token()),
        "the freed slot must generate tokens for the next request"
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
