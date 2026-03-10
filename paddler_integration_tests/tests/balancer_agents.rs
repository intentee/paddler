use std::time::Duration;

use integration_tests::BALANCER_INFERENCE_ADDR;
use integration_tests::BALANCER_MANAGEMENT_ADDR;
use integration_tests::balancer_params;
use integration_tests::managed_agent::ManagedAgent;
use integration_tests::managed_agent::ManagedAgentParams;
use integration_tests::managed_balancer::ManagedBalancer;
use serial_test::file_serial;
use tempfile::NamedTempFile;

#[tokio::test]
#[file_serial]
async fn test_balancer_can_register_agents() {
    let state_db = NamedTempFile::new().expect("failed to create temp file");

    let balancer = ManagedBalancer::spawn(balancer_params(
        BALANCER_MANAGEMENT_ADDR,
        BALANCER_INFERENCE_ADDR,
        state_db.path().to_str().unwrap(),
        30,
        Duration::from_secs(10),
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
