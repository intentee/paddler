use tokio::fs::try_exists;
use tokio_util::sync::CancellationToken;

use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::state_database_file::StateDatabaseFile;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn reading_a_missing_state_file_changes_nothing() {
    let database = StateDatabaseFile::new().expect("the state database file must be created");
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        desired_state: ClusterDesiredState::KeepStored,
        state_database_url: database.url.clone(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start without a state file");

    cluster
        .client_management
        .get_balancer_desired_state(CancellationToken::new())
        .await
        .expect("the balancer must report its desired state");

    assert!(
        !try_exists(&database.path)
            .await
            .expect("the state file path must be inspectable")
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
