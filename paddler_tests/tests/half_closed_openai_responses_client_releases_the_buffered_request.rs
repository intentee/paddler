use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::half_closed_client::HalfClosedClient;
use paddler_tests::start_cluster::start_cluster;
use serde_json::json;
use tokio::time::timeout;

const BUFFERED_REQUEST_TIMEOUT_LONGER_THAN_ANY_TEST_RUN: Duration = Duration::from_hours(1);
const RELEASE_OBSERVATION_WINDOW: Duration = Duration::from_secs(5);

#[tokio::test(flavor = "multi_thread")]
async fn half_closed_openai_responses_client_releases_the_buffered_request() -> Result<()> {
    let mut cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        buffered_request_timeout: BUFFERED_REQUEST_TIMEOUT_LONGER_THAN_ANY_TEST_RUN,
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await?;

    let mut client = HalfClosedClient::post_json_then_half_close(
        cluster.balancer.addresses.compat_openai,
        "/v1/responses",
        &json!({
            "input": "hi",
            "max_output_tokens": 2048,
            "model": "paddler",
            "stream": true,
        }),
    )
    .await?;

    cluster
        .wait_for_buffered_request_count(1)
        .await
        .context("the request must be buffered while no agent is available")?;

    client.half_close().await?;

    timeout(
        RELEASE_OBSERVATION_WINDOW,
        cluster.wait_for_buffered_request_count(0),
    )
    .await
    .context(
        "the OpenAI-compatible responses stream must release its buffered request when the \
         client half-closes",
    )??;

    drop(client);

    cluster.shutdown().await?;

    Ok(())
}
