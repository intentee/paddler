use anyhow::Result;
use paddler_bootstrap::bootstrap_error::BootstrapError;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::state_database_file::StateDatabaseFile;
use paddler_tests::start_cluster::start_cluster;
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_refuses_a_stored_desired_state_with_an_ambiguous_parameter() -> Result<()> {
    let database = StateDatabaseFile::new()?;
    let mut stored_desired_state = serde_json::to_value(BalancerDesiredState::default())?;

    stored_desired_state["inference_parameters"]["image_resize_to_fit"] = json!(0);

    tokio::fs::write(
        database.path(),
        serde_json::to_string(&json!({
            "balancer_desired_state": stored_desired_state,
            "version": "1",
        }))?,
    )
    .await?;

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

    Ok(())
}
