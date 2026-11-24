use tokio::fs::try_exists;
use tokio_util::sync::CancellationToken;

use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_cluster_harness::temporary_state_database_file::TemporaryStateDatabaseFile;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_keeping_its_stored_state_does_not_create_a_missing_state_file() {
    let database =
        TemporaryStateDatabaseFile::new().expect("the state database file must be created");
    let cluster = Cluster::start_in_process(ClusterParams {
        desired_state: ClusterDesiredState::KeepStored,
        state_database_url: database.url.clone(),
        ..ClusterParams::without_agents()
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
