#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::collect_generated_tokens::collect_generated_tokens;
use paddler_test_cluster_harness::observation_window::ObservationWindow;
use paddler_test_cluster_harness::token_result_with_producer::TokenResultWithProducer;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use tokio_util::sync::CancellationToken;

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
async fn inference_socket_burst_cancellation_serves_every_buffered_request() -> Result<()> {
    let mut cluster = start_cluster_with_qwen3(vec![AgentConfig::single(SLOT_COUNT)]).await?;
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
        .wait_for_slots_processing(&agent_id, SLOT_COUNT, ObservationWindow::model_load())
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
        .wait_for_buffered_request_count(WAITING_REQUEST_COUNT, ObservationWindow::model_load())
        .await?;

    for slot_filling_token in &slot_filling_tokens {
        slot_filling_token.cancel();
    }

    for mut slot_filling_stream in slot_filling_streams {
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
            "every buffered request must run to completion once a slot frees, not end with an error: {:?}",
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
        .wait_for_slots_processing(&agent_id, 0, ObservationWindow::model_load())
        .await?;
    cluster.shutdown().await?;

    Ok(())
}
