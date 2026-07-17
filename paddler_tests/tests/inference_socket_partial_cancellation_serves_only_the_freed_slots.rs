#![cfg(feature = "tests_that_use_llms")]

use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::collect_generated_tokens::collect_generated_tokens;
use paddler_test_cluster_harness::token_result_with_producer::TokenResultWithProducer;
use paddler_tests::qwen3_desired_state::qwen3_desired_state;
use paddler_tests::start_cluster::start_cluster;
use tokio_util::sync::CancellationToken;

const BUFFERED_REQUEST_TIMEOUT_LONGER_THAN_ANY_TEST_RUN: Duration = Duration::from_hours(1);
const CANCELLED_REQUEST_COUNT: usize = 2;
const SLOT_COUNT: i32 = 4;
const WAITING_REQUEST_COUNT: i32 = 4;

fn slot_filling_prompt() -> ContinueFromRawPromptParams {
    ContinueFromRawPromptParams {
        grammar: None,
        max_tokens: 500,
        raw_prompt: "Write a very long, detailed story about an explorer.".to_owned(),
    }
}

fn waiting_prompt() -> ContinueFromRawPromptParams {
    ContinueFromRawPromptParams {
        grammar: None,
        max_tokens: 32,
        raw_prompt: "The capital of France is".to_owned(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn inference_socket_partial_cancellation_serves_only_the_freed_slots() -> Result<()> {
    let mut cluster = start_cluster(ClusterParams {
        agents: vec![AgentConfig::single(SLOT_COUNT)],
        buffered_request_timeout: BUFFERED_REQUEST_TIMEOUT_LONGER_THAN_ANY_TEST_RUN,
        desired_state: Some(qwen3_desired_state()),
        wait_for_slots_ready: true,
        ..ClusterParams::default()
    })
    .await?;
    let agent_id = cluster
        .agent_ids
        .first()
        .context("cluster must have one registered agent")?
        .clone();

    let slot_filling_tokens: Vec<CancellationToken> =
        (0..SLOT_COUNT).map(|_| CancellationToken::new()).collect();
    let mut slot_filling_streams = Vec::new();

    for slot_filling_token in &slot_filling_tokens {
        slot_filling_streams.push(
            cluster
                .client_inference
                .continue_from_raw_prompt(slot_filling_token.clone(), slot_filling_prompt())
                .await
                .map_err(anyhow::Error::new)?,
        );
    }

    for slot_filling_stream in &mut slot_filling_streams {
        slot_filling_stream
            .next()
            .await
            .context("each slot-filling request must stream a message before it is cancelled")?
            .map_err(anyhow::Error::new)?;
    }

    cluster
        .wait_for_slots_processing(&agent_id, SLOT_COUNT)
        .await?;

    let mut waiting_streams = Vec::new();

    for _ in 0..WAITING_REQUEST_COUNT {
        waiting_streams.push(
            cluster
                .client_inference
                .continue_from_raw_prompt(CancellationToken::new(), waiting_prompt())
                .await
                .map_err(anyhow::Error::new)?,
        );
    }

    cluster
        .wait_for_buffered_request_count(WAITING_REQUEST_COUNT)
        .await?;

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
        let collected = collect_generated_tokens(waiting_stream).await?;

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
            SLOT_COUNT - i32::try_from(CANCELLED_REQUEST_COUNT)?,
        )
        .await?;

    for slot_filling_token in slot_filling_tokens.iter().skip(CANCELLED_REQUEST_COUNT) {
        slot_filling_token.cancel();
    }

    cluster.wait_for_slots_processing(&agent_id, 0).await?;
    cluster.shutdown().await?;

    Ok(())
}
