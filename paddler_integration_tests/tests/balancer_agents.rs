#![cfg(feature = "tests_that_use_compiled_paddler")]

use std::time::Duration;

use futures_util::StreamExt;
use integration_tests::BALANCER_INFERENCE_ADDR;
use integration_tests::BALANCER_MANAGEMENT_ADDR;
use integration_tests::balancer_params;
use integration_tests::managed_agent::ManagedAgent;
use integration_tests::managed_agent::ManagedAgentParams;
use integration_tests::managed_balancer::ManagedBalancer;
use integration_tests::test_cluster::TestCluster;
use integration_tests::test_cluster_params::TestClusterParams;
use serial_test::file_serial;
use tempfile::NamedTempFile;

async fn get_first_agent_id(balancer: &ManagedBalancer) -> String {
    let snapshot = balancer
        .client()
        .management()
        .get_agents()
        .await
        .expect("failed to get agents");

    snapshot
        .agents
        .first()
        .expect("should have at least one agent")
        .id
        .clone()
}

#[tokio::test]
#[file_serial]
async fn test_balancer_can_register_agents() {
    let state_db = NamedTempFile::new().expect("failed to create temp file");
    let state_db_url = format!("file://{}", state_db.path().to_str().unwrap());

    let balancer = ManagedBalancer::spawn(balancer_params(
        BALANCER_MANAGEMENT_ADDR,
        BALANCER_INFERENCE_ADDR,
        &state_db_url,
        30,
        Duration::from_secs(10),
    ))
    .await
    .expect("failed to spawn balancer");

    let agent_count = balancer.wait_for_agent_count(0).await;

    assert_eq!(agent_count, 0);

    let _agent1 = ManagedAgent::spawn(&ManagedAgentParams {
        management_addr: BALANCER_MANAGEMENT_ADDR.to_string(),
        name: Some("test-agent-1".to_string()),
        slots: 1,
    })
    .expect("failed to spawn agent");

    let agent_count = balancer.wait_for_agent_count(1).await;

    assert_eq!(agent_count, 1);

    let _agent2 = ManagedAgent::spawn(&ManagedAgentParams {
        management_addr: BALANCER_MANAGEMENT_ADDR.to_string(),
        name: Some("test-agent-2".to_string()),
        slots: 1,
    })
    .expect("failed to spawn agent");

    let agent_count = balancer.wait_for_agent_count(2).await;

    assert_eq!(agent_count, 2);
}

#[tokio::test]
#[file_serial]
async fn test_agents_stream_receives_snapshot() {
    let cluster = TestCluster::spawn(TestClusterParams {
        agent_name: "agents-test-agent".to_string(),
        agent_slots: 2,
        ..TestClusterParams::default()
    })
    .await
    .expect("failed to spawn cluster");

    let mut stream = cluster
        .balancer
        .client()
        .management()
        .agents_stream()
        .await
        .expect("agents stream should connect");

    let first_event = stream
        .next()
        .await
        .expect("stream must produce at least one event")
        .expect("first event should deserialize");

    assert!(
        !first_event.agents.is_empty(),
        "first stream event must contain agents"
    );
}

#[tokio::test]
#[file_serial]
async fn test_get_model_metadata() {
    let cluster = TestCluster::spawn(TestClusterParams {
        agent_name: "agents-test-agent".to_string(),
        agent_slots: 2,
        ..TestClusterParams::default()
    })
    .await
    .expect("failed to spawn cluster");

    let agent_id = get_first_agent_id(&cluster.balancer).await;

    let metadata = cluster
        .balancer
        .client()
        .management()
        .get_model_metadata(&agent_id)
        .await
        .expect("get_model_metadata should succeed");

    assert!(
        metadata.is_some(),
        "model metadata should be present for a loaded model"
    );

    assert!(
        !metadata
            .expect("metadata should be present")
            .metadata
            .is_empty(),
        "model metadata map should not be empty"
    );
}

#[tokio::test]
#[file_serial]
async fn test_get_metrics_returns_prometheus_format() {
    let cluster = TestCluster::spawn(TestClusterParams {
        agent_name: "agents-test-agent".to_string(),
        agent_slots: 2,
        ..TestClusterParams::default()
    })
    .await
    .expect("failed to spawn cluster");

    let metrics = cluster
        .balancer
        .client()
        .management()
        .get_metrics()
        .await
        .expect("get_metrics should succeed");

    assert!(
        metrics.contains("slots_processing"),
        "metrics must contain slots_processing gauge"
    );

    assert!(
        metrics.contains("slots_total"),
        "metrics must contain slots_total gauge"
    );
}

#[tokio::test]
#[file_serial]
async fn test_agent_reports_download_progress() {
    let cluster = TestCluster::spawn(TestClusterParams {
        agent_name: "download-progress-agent".to_string(),
        agent_slots: 2,
        ..TestClusterParams::default()
    })
    .await
    .expect("failed to spawn cluster");

    let snapshot = cluster
        .balancer
        .client()
        .management()
        .get_agents()
        .await
        .expect("failed to get agents");

    assert!(
        !snapshot.agents.is_empty(),
        "should have at least one agent after startup"
    );

    let agent = &snapshot.agents[0];

    assert_eq!(
        agent.download_current, 0,
        "download_current should be 0 after model is loaded"
    );

    assert_eq!(
        agent.download_total, 0,
        "download_total should be 0 after model is loaded"
    );
}
