use anyhow::Context as _;
use anyhow::Result;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::half_closed_client::HalfClosedClient;
use paddler_test_cluster_harness::observation_window::ObservationWindow;
use paddler_tests::start_cluster::start_cluster;
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn half_closed_openai_non_streaming_client_releases_the_buffered_request() -> Result<()> {
    let mut cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::without_request_expiry()
    })
    .await?;

    let mut client = HalfClosedClient::post_json_then_half_close(
        cluster.balancer.addresses.compat_openai,
        "/v1/chat/completions",
        &json!({
            "max_completion_tokens": 2048,
            "messages": [{"role": "user", "content": "hi"}],
            "model": "paddler",
            "stream": false,
        }),
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
        .context(
            "a non-streaming OpenAI-compatible request buffers its response inside the handler, so \
         the balancer must still release it when the client half-closes",
        )?;

    drop(client);

    cluster.shutdown().await?;

    Ok(())
}
