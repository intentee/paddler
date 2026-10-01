use tokio_util::sync::CancellationToken;

use paddler_cli_tests::start_subprocess_cluster::start_subprocess_cluster;
use paddler_client::reports_health::ReportsHealth as _;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_serves_on_the_addresses_it_announces() {
    let cluster = start_subprocess_cluster(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        ClusterParams {
            agents: Vec::new(),
            wait_for_slots_ready: false,
            ..ClusterParams::default()
        },
    )
    .await
    .expect("the balancer subprocess must start and announce its addresses");

    let inference_health = cluster
        .client_inference
        .get_health(CancellationToken::new())
        .await
        .expect("the announced inference address must serve health checks");
    let compat_openai_health = cluster
        .client_compat_openai_health
        .get_health(CancellationToken::new())
        .await
        .expect("the announced OpenAI compatibility address must serve health checks");

    assert_eq!(inference_health, "OK");
    assert_eq!(compat_openai_health, "OK");

    cluster
        .shutdown()
        .await
        .expect("the subprocess cluster must shut down cleanly");
}
