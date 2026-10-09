use std::fs::Permissions;
use std::fs::set_permissions;
use std::os::unix::fs::PermissionsExt as _;

use reqwest::StatusCode;
use tempfile::TempDir;

use paddler_cli_tests::start_subprocess_cluster::start_subprocess_cluster;
use paddler_client::error::Error as ClientError;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

const READ_ONLY_FILE_MODE: u32 = 0o444;

fn cluster_params_storing_state_in(state_database_url: String) -> ClusterParams {
    ClusterParams {
        agents: Vec::new(),
        state_database_url,
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn subprocess_cluster_reports_a_desired_state_the_balancer_cannot_store() {
    let state_directory = TempDir::new().expect("a temporary directory must be created");
    let state_file_path = state_directory.path().join("state.json");
    let state_database_url = format!("file://{}", state_file_path.display());

    start_subprocess_cluster(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        cluster_params_storing_state_in(state_database_url.clone()),
    )
    .await
    .expect("the first balancer must create its state file")
    .shutdown()
    .await
    .expect("the first cluster must shut down cleanly");

    set_permissions(
        &state_file_path,
        Permissions::from_mode(READ_ONLY_FILE_MODE),
    )
    .expect("the state file must become read-only");

    let start_error = start_subprocess_cluster(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        cluster_params_storing_state_in(state_database_url),
    )
    .await
    .err()
    .expect("a desired state the balancer cannot store must fail the cluster start");

    assert!(matches!(
        start_error.downcast_ref::<ClientError>(),
        Some(ClientError::UnexpectedResponseStatus { status, .. })
            if *status == StatusCode::INTERNAL_SERVER_ERROR
    ));
}
