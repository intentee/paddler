use std::error::Error as _;
use std::mem::discriminant;

use serde_json::Value;
use serde_json::from_str;
use serde_json::json;
use serde_json::to_string;
use serde_json::to_value;
use tokio::fs::write;

use paddler_bootstrap::bootstrap_error::BootstrapError;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_state_database::state_database_error::StateDatabaseError;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::temporary_state_database_file::TemporaryStateDatabaseFile;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_refuses_a_stored_desired_state_with_an_ambiguous_parameter() {
    let database =
        TemporaryStateDatabaseFile::new().expect("the state database file must be created");
    let mut stored_desired_state =
        to_value(BalancerDesiredState::default()).expect("the value must serialize");

    stored_desired_state["text_generation"]["multimodal"]["image_resize_to_fit"] = json!(0);

    write(
        &database.path,
        to_string(&json!({
            "balancer_desired_state": stored_desired_state,
            "version": "2",
        }))
        .expect("the value must serialize"),
    )
    .await
    .expect("the file must be written");

    let start_error = start_cluster(ClusterParams {
        state_database_url: database.url.clone(),
        desired_state: ClusterDesiredState::KeepStored,
        ..ClusterParams::without_agents()
    })
    .await
    .err();

    let bootstrap_error = start_error
        .as_ref()
        .and_then(|error| error.downcast_ref::<BootstrapError>());

    assert_eq!(
        bootstrap_error.map(discriminant),
        Some(discriminant(&BootstrapError::StateDatabaseReadFailed {
            source: StateDatabaseError::FilePathEmpty
        })),
        "{start_error:?}"
    );
    assert_eq!(
        bootstrap_error
            .and_then(|bootstrap_error| bootstrap_error.source())
            .map(ToString::to_string),
        Some(
            StateDatabaseError::FileContentsInvalid {
                path: database.path.clone(),
                source: from_str::<Value>("")
                    .expect_err("an empty document must not parse as JSON"),
            }
            .to_string()
        )
    );
}
