use std::time::Duration;

use integration_tests::BALANCER_INFERENCE_ADDR;
use integration_tests::BALANCER_MANAGEMENT_ADDR;
use integration_tests::managed_agent::ManagedAgent;
use integration_tests::managed_agent::ManagedAgentParams;
use integration_tests::managed_balancer::ManagedBalancer;
use integration_tests::managed_balancer::ManagedBalancerParams;
use serial_test::file_serial;
use tempfile::NamedTempFile;

fn default_balancer_params(
    management_addr: &str,
    inference_addr: &str,
    state_database_path: &str,
) -> ManagedBalancerParams {
    ManagedBalancerParams {
        buffered_request_timeout: Duration::from_secs(10),
        inference_addr: inference_addr.to_string(),
        management_addr: management_addr.to_string(),
        max_buffered_requests: 30,
        state_database_path: state_database_path.to_string(),
    }
}

#[tokio::test]
#[file_serial]
async fn test_balancer_can_register_agents() {
    let state_db = NamedTempFile::new().expect("failed to create temp file");

    let balancer = ManagedBalancer::spawn(default_balancer_params(
        BALANCER_MANAGEMENT_ADDR,
        BALANCER_INFERENCE_ADDR,
        state_db.path().to_str().unwrap(),
    ))
    .await
    .expect("failed to spawn balancer");

    let agent_count = balancer.wait_for_agent_count(0).await;

    assert_eq!(agent_count, 0);

    let _agent1 = ManagedAgent::spawn(ManagedAgentParams {
        management_addr: BALANCER_MANAGEMENT_ADDR.to_string(),
        name: Some("test-agent-1".to_string()),
        slots: 1,
    })
    .await
    .expect("failed to spawn agent");

    let agent_count = balancer.wait_for_agent_count(1).await;

    assert_eq!(agent_count, 1);

    let _agent2 = ManagedAgent::spawn(ManagedAgentParams {
        management_addr: BALANCER_MANAGEMENT_ADDR.to_string(),
        name: Some("test-agent-2".to_string()),
        slots: 1,
    })
    .await
    .expect("failed to spawn agent");

    let agent_count = balancer.wait_for_agent_count(2).await;

    assert_eq!(agent_count, 2);
}
