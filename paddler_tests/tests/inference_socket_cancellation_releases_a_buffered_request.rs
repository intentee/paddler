use std::time::Duration;

use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;
use tokio_util::sync::CancellationToken;

const BUFFERED_REQUEST_TIMEOUT_LONGER_THAN_ANY_TEST_RUN: Duration = Duration::from_hours(1);

#[tokio::test(flavor = "multi_thread")]
async fn inference_socket_cancellation_releases_a_buffered_request() -> Result<()> {
    let mut cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        buffered_request_timeout: BUFFERED_REQUEST_TIMEOUT_LONGER_THAN_ANY_TEST_RUN,
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await?;

    let cancellation_token = CancellationToken::new();

    let mut stream = cluster
        .client_inference
        .continue_from_raw_prompt(
            cancellation_token.clone(),
            ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: 16,
                raw_prompt: "The capital of France is".to_owned(),
            },
        )
        .await
        .map_err(anyhow::Error::new)?;

    cluster.wait_for_buffered_request_count(1).await?;

    cancellation_token.cancel();

    assert!(
        stream.next().await.is_none(),
        "a cancelled inference socket request must end its stream"
    );

    cluster.wait_for_buffered_request_count(0).await?;

    cluster.shutdown().await?;

    Ok(())
}
