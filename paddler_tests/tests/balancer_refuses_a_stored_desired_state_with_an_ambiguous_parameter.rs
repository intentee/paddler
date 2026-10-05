use serde_json::json;
use serde_json::to_string;
use serde_json::to_value;
use tokio::fs::write;

use paddler_bootstrap::bootstrap_error::BootstrapError;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::state_database_file::StateDatabaseFile;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_refuses_a_stored_desired_state_with_an_ambiguous_parameter() {
    let database = StateDatabaseFile::new().expect("the state database file must be created");
    let mut stored_desired_state =
        to_value(BalancerDesiredState::default()).expect("the value must serialize");

    stored_desired_state["inference_parameters"]["image_resize_to_fit"] = json!(0);

    write(
        &database.path,
        to_string(&json!({
            "balancer_desired_state": stored_desired_state,
            "version": "1",
        }))
        .expect("the value must serialize"),
    )
    .await
    .expect("the file must be written");

    let start_error = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        state_database_url: database.url.clone(),
        desired_state: None,
        ..ClusterParams::default()
    })
    .await
    .err();

    assert!(
        matches!(
            start_error
                .as_ref()
                .and_then(|error| error.downcast_ref::<BootstrapError>()),
            Some(BootstrapError::StateDatabaseReadFailed { .. })
        ),
        "{start_error:?}"
    );
}
