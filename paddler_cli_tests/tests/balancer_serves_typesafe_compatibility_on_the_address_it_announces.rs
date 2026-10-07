use tokio_util::sync::CancellationToken;

use paddler_cli_tests::start_subprocess_cluster::start_subprocess_cluster;
use paddler_client::reports_health::ReportsHealth as _;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_serves_typesafe_compatibility_on_the_address_it_announces() {
    let cluster = start_subprocess_cluster(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        ClusterParams {
            agents: Vec::new(),
            desired_state: ClusterDesiredState::KeepStored(InferenceMode::Decision),
            wait_for_slots_ready: false,
            ..ClusterParams::default()
        },
    )
    .await
    .expect("the decision balancer subprocess must start and announce its addresses");

    let compat_typesafe_health = cluster
        .compat_typesafe_health_client()
        .expect("a decision cluster must serve TypeSafe compatibility")
        .get_health(CancellationToken::new())
        .await
        .expect("the announced TypeSafe compatibility address must serve health checks");

    assert_eq!(compat_typesafe_health, "OK");

    cluster
        .shutdown()
        .await
        .expect("the subprocess cluster must shut down cleanly");
}
