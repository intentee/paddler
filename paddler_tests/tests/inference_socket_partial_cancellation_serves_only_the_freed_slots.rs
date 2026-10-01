#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::collect_generated_tokens::collect_generated_tokens;
use paddler_test_cluster_harness::token_result_with_producer::TokenResultWithProducer;
use paddler_test_cluster_harness::unending_generation::unending_generation;
use paddler_tests::qwen3_desired_state::qwen3_desired_state;
use paddler_tests::start_cluster::start_cluster;

const WAITING_MAX_TOKENS: NonZeroU32 = NonZeroU32::new(32).unwrap();
const CANCELLED_REQUEST_COUNT: usize = 2;
const SLOT_COUNT: u16 = 4;
const WAITING_REQUEST_COUNT: u64 = 4;

fn waiting_prompt() -> ContinueFromRawPromptParams {
    ContinueFromRawPromptParams {
        grammar: None,
        max_tokens: WAITING_MAX_TOKENS,
        raw_prompt: "The capital of France is".to_owned(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn inference_socket_partial_cancellation_serves_only_the_freed_slots() {
    let mut cluster = start_cluster(ClusterParams {
        agents: vec![AgentConfig::single(SLOT_COUNT)],
        desired_state: Some(qwen3_desired_state()),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");
    let agent_id = cluster
        .agent_ids
        .first()
        .expect("cluster must have one registered agent")
        .clone();

    let slot_filling_tokens: Vec<CancellationToken> = (0..SLOT_COUNT)
        .map(|_slot_index| CancellationToken::new())
        .collect();
    let mut slot_filling_streams = Vec::new();

    for slot_filling_token in &slot_filling_tokens {
        slot_filling_streams.push(
            cluster
                .client_inference
                .continue_from_raw_prompt(slot_filling_token.clone(), unending_generation())
                .await
                .expect("the inference request must be accepted"),
        );
    }

    for slot_filling_stream in &mut slot_filling_streams {
        slot_filling_stream
            .next()
            .await
            .expect("each slot-filling request must stream a message before it is cancelled")
            .expect("the message must be readable");
    }

    cluster
        .wait_for_slots_processing(&agent_id, u64::from(SLOT_COUNT))
        .await
        .expect("the agent must reach the expected slot usage");

    let mut waiting_streams = Vec::new();

    for _ in 0..WAITING_REQUEST_COUNT {
        waiting_streams.push(
            cluster
                .client_inference
                .continue_from_raw_prompt(CancellationToken::new(), waiting_prompt())
                .await
                .expect("the inference request must be accepted"),
        );
    }

    cluster
        .wait_for_buffered_request_count(WAITING_REQUEST_COUNT)
        .await
        .expect("the balancer must reach the expected buffered request count");

    for slot_filling_token in slot_filling_tokens.iter().take(CANCELLED_REQUEST_COUNT) {
        slot_filling_token.cancel();
    }

    for slot_filling_stream in slot_filling_streams
        .iter_mut()
        .take(CANCELLED_REQUEST_COUNT)
    {
        assert!(
            slot_filling_stream.next().await.is_none(),
            "a cancelled request must end its stream"
        );
    }

    for waiting_stream in waiting_streams {
        let collected = collect_generated_tokens(waiting_stream)
            .await
            .expect("the generated tokens must be collected");

        assert!(
            matches!(
                collected.token_results.last(),
                Some(TokenResultWithProducer {
                    token_result: GeneratedTokenResult::Done(_),
                    ..
                })
            ),
            "every buffered request must run to completion through the freed slots, not end with an error: {:?}",
            collected.token_results.last()
        );
        assert!(
            collected
                .token_results
                .iter()
                .any(|token_result_with_producer| token_result_with_producer
                    .token_result
                    .is_token()),
            "every buffered request must be served, not finished without generating a single token"
        );
    }

    cluster
        .wait_for_slots_processing(
            &agent_id,
            u64::from(SLOT_COUNT)
                - u64::try_from(CANCELLED_REQUEST_COUNT)
                    .expect("the value must fit its target type"),
        )
        .await
        .expect("the agent must reach the expected slot usage");

    for slot_filling_token in slot_filling_tokens.iter().skip(CANCELLED_REQUEST_COUNT) {
        slot_filling_token.cancel();
    }

    for slot_filling_stream in slot_filling_streams
        .iter_mut()
        .skip(CANCELLED_REQUEST_COUNT)
    {
        assert!(
            slot_filling_stream.next().await.is_none(),
            "a cancelled request must end its stream"
        );
    }

    cluster
        .wait_for_slots_processing(&agent_id, 0)
        .await
        .expect("the agent must reach the expected slot usage");
    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
