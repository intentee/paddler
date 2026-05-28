use anyhow::Context as _;
use anyhow::Result;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::agents_status::assert_agent_count::assert_agent_count;
use paddler_tests::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_registers_multiple_agents_over_time() -> Result<()> {
    let mut cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await?;

    cluster.spawn_additional_agent(&AgentConfig {
        name: "test-agent-1".to_owned(),
        slot_count: 1,
    });

    cluster
        .agents
        .until(assert_agent_count(1))
        .await
        .context("first agent should register")?;

    cluster.spawn_additional_agent(&AgentConfig {
        name: "test-agent-2".to_owned(),
        slot_count: 1,
    });

    cluster
        .agents
        .until(assert_agent_count(2))
        .await
        .context("second agent should register")?;

    cluster.shutdown().await?;

    Ok(())
}
