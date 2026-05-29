use anyhow::Context as _;
use anyhow::Result;
use paddler_cli_tests::agent_config::AgentConfig;
use paddler_cli_tests::cluster_params::ClusterParams;
use paddler_cli_tests::start_subprocess_cluster::start_subprocess_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_registers_multiple_agents_over_time() -> Result<()> {
    let mut cluster = start_subprocess_cluster(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        ClusterParams {
            agents: Vec::new(),
            wait_for_slots_ready: false,
            ..ClusterParams::default()
        },
    )
    .await?;

    cluster.spawn_additional_agent(&AgentConfig {
        name: "test-agent-1".to_owned(),
        slot_count: 1,
    })?;

    cluster
        .wait_for_agent_count(1)
        .await
        .context("first agent should register")?;

    cluster.spawn_additional_agent(&AgentConfig {
        name: "test-agent-2".to_owned(),
        slot_count: 1,
    })?;

    cluster
        .wait_for_agent_count(2)
        .await
        .context("second agent should register")?;

    cluster.shutdown().await?;

    Ok(())
}
