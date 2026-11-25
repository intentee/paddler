use tokio_util::sync::CancellationToken;

use paddler_client::reports_health::ReportsHealth as _;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_params::ClusterParams;

#[tokio::test(flavor = "multi_thread")]
async fn openai_health_returns_ok() {
    let cluster = Cluster::start_in_process(ClusterParams::without_agents())
        .await
        .expect("the cluster must start");

    let health = cluster
        .compat_openai_health_client()
        .expect("a text generation cluster must serve OpenAI compatibility")
        .get_health(CancellationToken::new())
        .await
        .expect("failed to GET OpenAI compat /health");

    assert_eq!(health, "OK");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
