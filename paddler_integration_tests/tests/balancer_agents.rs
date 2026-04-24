#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt;
use paddler_integration_tests::managed_agent::ManagedAgent;
use paddler_integration_tests::managed_agent_params::ManagedAgentParams;
use paddler_integration_tests::managed_balancer::ManagedBalancer;
use paddler_integration_tests::managed_balancer_params::ManagedBalancerParams;
use paddler_integration_tests::managed_cluster::ManagedCluster;
use paddler_integration_tests::managed_cluster_params::ManagedClusterParams;
use paddler_integration_tests::pick_balancer_addresses::pick_balancer_addresses;
use serial_test::file_serial;
use tempfile::NamedTempFile;

async fn get_first_agent_id(balancer: &ManagedBalancer) -> Result<String> {
    let snapshot = balancer
        .client()
        .management()
        .get_agents()
        .await
        .context("failed to get agents")?;

    Ok(snapshot
        .agents
        .first()
        .context("should have at least one agent")?
        .id
        .clone())
}

#[tokio::test]
#[file_serial]
async fn test_balancer_can_register_agents() -> Result<()> {
    let state_db = NamedTempFile::new().context("failed to create temp file")?;
    let state_db_url = format!(
        "file://{}",
        state_db
            .path()
            .to_str()
            .context("temp file path is not valid UTF-8")?
    );
    let addresses = pick_balancer_addresses().context("pick addresses")?;

    let balancer = ManagedBalancer::spawn(ManagedBalancerParams {
        buffered_request_timeout: Duration::from_secs(10),
        compat_openai_addr: addresses.compat_openai,
        inference_addr: addresses.inference,
        inference_cors_allowed_hosts: vec![],
        inference_item_timeout: None,
        management_addr: addresses.management.clone(),
        management_cors_allowed_hosts: vec![],
        max_buffered_requests: 30,
        state_database_url: state_db_url,
    })
    .await
    .context("failed to spawn balancer")?;

    let agent_count = balancer.wait_for_agent_count(0).await;

    assert_eq!(agent_count, 0);

    let _agent1 = ManagedAgent::spawn(&ManagedAgentParams {
        management_addr: addresses.management.clone(),
        name: Some("test-agent-1".to_owned()),
        slots: 1,
    })
    .context("failed to spawn agent")?;

    let agent_count = balancer.wait_for_agent_count(1).await;

    assert_eq!(agent_count, 1);

    let _agent2 = ManagedAgent::spawn(&ManagedAgentParams {
        management_addr: addresses.management,
        name: Some("test-agent-2".to_owned()),
        slots: 1,
    })
    .context("failed to spawn agent")?;

    let agent_count = balancer.wait_for_agent_count(2).await;

    assert_eq!(agent_count, 2);

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_agents_stream_receives_snapshot() -> Result<()> {
    let cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_name: "agents-test-agent".to_owned(),
        agent_slots: 2,
        ..ManagedClusterParams::default()
    })
    .await
    .context("failed to spawn cluster")?;

    let mut stream = cluster
        .balancer
        .client()
        .management()
        .get_agents_stream()
        .await
        .context("agents stream should connect")?;

    let first_event = stream
        .next()
        .await
        .context("stream must produce at least one event")?
        .context("first event should deserialize")?;

    assert!(
        !first_event.agents.is_empty(),
        "first stream event must contain agents"
    );

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_get_model_metadata() -> Result<()> {
    let cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_name: "agents-test-agent".to_owned(),
        agent_slots: 2,
        ..ManagedClusterParams::default()
    })
    .await
    .context("failed to spawn cluster")?;

    let agent_id = get_first_agent_id(&cluster.balancer).await?;

    let metadata = cluster
        .balancer
        .client()
        .management()
        .get_model_metadata(&agent_id)
        .await
        .context("get_model_metadata should succeed")?;

    assert!(
        metadata.is_some(),
        "model metadata should be present for a loaded model"
    );

    assert!(
        !metadata
            .context("metadata should be present")?
            .metadata
            .is_empty(),
        "model metadata map should not be empty"
    );

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_get_metrics_returns_prometheus_format() -> Result<()> {
    let cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_name: "agents-test-agent".to_owned(),
        agent_slots: 2,
        ..ManagedClusterParams::default()
    })
    .await
    .context("failed to spawn cluster")?;

    let metrics = cluster
        .balancer
        .client()
        .management()
        .get_metrics()
        .await
        .context("get_metrics should succeed")?;

    assert!(
        metrics.contains("slots_processing"),
        "metrics must contain slots_processing gauge"
    );

    assert!(
        metrics.contains("slots_total"),
        "metrics must contain slots_total gauge"
    );

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_agent_reports_download_progress() -> Result<()> {
    let cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_name: "download-progress-agent".to_owned(),
        agent_slots: 2,
        ..ManagedClusterParams::default()
    })
    .await
    .context("failed to spawn cluster")?;

    let snapshot = cluster
        .balancer
        .client()
        .management()
        .get_agents()
        .await
        .context("failed to get agents")?;

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

    Ok(())
}
