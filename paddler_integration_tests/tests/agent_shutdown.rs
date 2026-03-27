#![cfg(feature = "tests_that_use_compiled_paddler")]

use std::time::Duration;

use paddler_integration_tests::BALANCER_INFERENCE_ADDR;
use paddler_integration_tests::BALANCER_MANAGEMENT_ADDR;
use paddler_integration_tests::BALANCER_OPENAI_ADDR;
use paddler_integration_tests::managed_agent::ManagedAgent;
use paddler_integration_tests::managed_agent::ManagedAgentParams;
use paddler_integration_tests::managed_balancer::ManagedBalancer;
use paddler_integration_tests::managed_balancer::ManagedBalancerParams;
use serial_test::file_serial;
use tempfile::NamedTempFile;

#[tokio::test]
#[file_serial]
async fn test_agent_shuts_down_gracefully_without_reconnecting() {
    let state_db = NamedTempFile::new().expect("failed to create temp file");
    let state_db_url = format!("file://{}", state_db.path().to_str().unwrap());

    let balancer = ManagedBalancer::spawn(ManagedBalancerParams {
        buffered_request_timeout: Duration::from_secs(10),
        compat_openai_addr: BALANCER_OPENAI_ADDR.to_string(),
        inference_addr: BALANCER_INFERENCE_ADDR.to_string(),
        inference_cors_allowed_hosts: vec![],
        inference_item_timeout: None,
        management_addr: BALANCER_MANAGEMENT_ADDR.to_string(),
        management_cors_allowed_hosts: vec![],
        max_buffered_requests: 30,
        state_database_url: state_db_url,
    })
    .await
    .expect("failed to spawn balancer");

    let mut agent = ManagedAgent::spawn(&ManagedAgentParams {
        management_addr: BALANCER_MANAGEMENT_ADDR.to_string(),
        name: Some("test-agent".to_string()),
        slots: 1,
    })
    .expect("failed to spawn agent");

    balancer.wait_for_agent_count(1).await;

    let exited_cleanly = agent.graceful_shutdown(Duration::from_secs(5)).await;

    assert!(
        exited_cleanly,
        "Agent did not exit within 5 seconds after SIGTERM — \
         likely reconnected after shutdown"
    );

    balancer.wait_for_agent_count(0).await;
}
