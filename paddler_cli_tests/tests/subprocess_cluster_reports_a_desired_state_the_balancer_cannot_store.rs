use std::fs::Permissions;
use std::fs::set_permissions;
use std::os::unix::fs::PermissionsExt as _;

use reqwest::StatusCode;
use tempfile::TempDir;
use url::Url;

use paddler_cli_tests::start_subprocess_cluster::start_subprocess_cluster;
use paddler_client::error::Error as ClientError;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::api_path::ApiPath;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;

const READ_ONLY_DIRECTORY_MODE: u32 = 0o555;

#[tokio::test(flavor = "multi_thread")]
async fn subprocess_cluster_reports_a_desired_state_the_balancer_cannot_store() {
    let state_directory = TempDir::new().expect("a temporary directory must be created");

    set_permissions(
        state_directory.path(),
        Permissions::from_mode(READ_ONLY_DIRECTORY_MODE),
    )
    .expect("the state directory must become read-only");

    let start_error = start_subprocess_cluster(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        ClusterParams {
            desired_state: ClusterDesiredState::Apply(Box::new(BalancerDesiredState {
                model: AgentDesiredModel::Uri("agent:///unstorable-model.gguf".to_owned()),
                ..BalancerDesiredState::default()
            })),
            state_database_url: format!(
                "file://{}",
                state_directory.path().join("state.json").display()
            ),
            ..ClusterParams::without_agents()
        },
    )
    .await
    .err()
    .expect("a desired state the balancer cannot store must fail the cluster start");

    assert!(matches!(
        start_error.downcast_ref::<ClientError>(),
        Some(ClientError::UnexpectedResponseStatus { message, status, url })
            if *status == StatusCode::INTERNAL_SERVER_ERROR
                && *message
                    == format!(
                        "Unable to create a replacement of the state database file in '{}'",
                        state_directory.path().display()
                    )
                && Url::parse(url).is_ok_and(|desired_state_url| {
                    desired_state_url.path() == ApiPath::BALANCER_DESIRED_STATE
                })
    ));
}
