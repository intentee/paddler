#![cfg(feature = "tests_that_use_docker")]

use anyhow::Result;
use paddler_cluster::agent_config::AgentConfig;
use paddler_cluster::cluster::Cluster;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_testcontainer::container_cluster_backend::ContainerClusterBackend;

#[tokio::test(flavor = "multi_thread")]
async fn container_cluster_reports_healthy_and_registers_agent() -> Result<()> {
    let cluster = Cluster::start(
        &ContainerClusterBackend,
        ClusterParams {
            agents: vec![AgentConfig::single(2)],
            desired_state: None,
            wait_for_slots_ready: false,
        },
    )
    .await?;

    let management_health = cluster
        .management_client
        .health()
        .await
        .map_err(anyhow::Error::new)?;
    assert_eq!(management_health, "OK");

    let inference_health = cluster
        .inference_client
        .health()
        .await
        .map_err(anyhow::Error::new)?;
    assert_eq!(inference_health, "OK");

    let agents = cluster
        .management_client
        .agents()
        .await
        .map_err(anyhow::Error::new)?;
    assert_eq!(
        agents.agents.len(),
        1,
        "the single agent container must register with the balancer"
    );

    cluster.shutdown().await?;

    Ok(())
}
