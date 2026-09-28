use anyhow::Context as _;
use anyhow::Result;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::half_closed_client::HalfClosedClient;
use paddler_test_cluster_harness::observation_window::ObservationWindow;
use serde_json::Value;

use crate::start_cluster::start_cluster;

pub async fn release_half_closed_openai_request(path: &str, body: &Value) -> Result<()> {
    let mut cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::without_request_expiry()
    })
    .await?;
    let mut client = HalfClosedClient::post_json_then_half_close(
        cluster.balancer.compat_openai_addr()?,
        path,
        body,
    )
    .await?;

    cluster
        .wait_for_buffered_request_count(1, ObservationWindow::model_load())
        .await
        .context("the request must be buffered while no agent is available")?;
    client.half_close().await?;
    cluster
        .wait_for_buffered_request_count(0, ObservationWindow::release())
        .await
        .context("the balancer must release the buffered request once the client half-closes")?;

    drop(client);

    cluster.shutdown().await
}
