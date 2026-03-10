use std::time::Duration;

use integration_tests::AGENT_DESIRED_MODEL;
use integration_tests::BALANCER_INFERENCE_ADDR;
use integration_tests::BALANCER_MANAGEMENT_ADDR;
use integration_tests::balancer_params;
use integration_tests::managed_balancer::ManagedBalancer;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::inference_parameters::InferenceParameters;
use serial_test::file_serial;
use tempfile::NamedTempFile;

#[tokio::test]
#[file_serial]
async fn test_desired_state_persists_across_restarts() {
    let state_db = NamedTempFile::new().expect("failed to create temp file");
    let state_db_path = state_db.path().to_str().unwrap().to_string();

    let desired_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters::default(),
        model: AGENT_DESIRED_MODEL.clone(),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: false,
    };

    let mut balancer = ManagedBalancer::spawn(balancer_params(
        BALANCER_MANAGEMENT_ADDR,
        BALANCER_INFERENCE_ADDR,
        &state_db_path,
        30,
        Duration::from_secs(10),
    ))
    .await
    .expect("failed to spawn first balancer");

    balancer
        .client()
        .management()
        .put_balancer_desired_state(&desired_state)
        .await
        .expect("failed to set balancer desired state");

    balancer.wait_for_desired_state(&desired_state).await;

    balancer
        .shutdown()
        .await
        .expect("failed to shutdown first balancer");

    let restarted_balancer = ManagedBalancer::spawn(balancer_params(
        BALANCER_MANAGEMENT_ADDR,
        BALANCER_INFERENCE_ADDR,
        &state_db_path,
        30,
        Duration::from_secs(10),
    ))
    .await
    .expect("failed to spawn second balancer");

    restarted_balancer
        .wait_for_desired_state(&desired_state)
        .await;
}
