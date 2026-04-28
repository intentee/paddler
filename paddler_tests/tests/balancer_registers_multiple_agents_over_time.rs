#![cfg(feature = "tests_that_use_compiled_paddler")]

use anyhow::Context as _;
use anyhow::Result;
use paddler_tests::agents_status::assert_agent_count::assert_agent_count;
use paddler_tests::spawn_agent_subprocess::spawn_agent_subprocess;
use paddler_tests::spawn_agent_subprocess_params::SpawnAgentSubprocessParams;
use paddler_tests::start_subprocess_cluster::start_subprocess_cluster;
use paddler_tests::subprocess_cluster_params::SubprocessClusterParams;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_registers_multiple_agents_over_time() -> Result<()> {
    let mut cluster = start_subprocess_cluster(SubprocessClusterParams {
        agent_count: 0,
        wait_for_slots_ready: false,
        ..SubprocessClusterParams::default()
    })
    .await?;

    let mut first_agent = spawn_agent_subprocess(SpawnAgentSubprocessParams {
        management_addr: cluster.addresses.management,
        name: Some("test-agent-1".to_owned()),
        slots: 1,
    })?;

    cluster
        .agents
        .until(assert_agent_count(1))
        .await
        .context("first agent should register")?;

    let mut second_agent = spawn_agent_subprocess(SpawnAgentSubprocessParams {
        management_addr: cluster.addresses.management,
        name: Some("test-agent-2".to_owned()),
        slots: 1,
    })?;

    cluster
        .agents
        .until(assert_agent_count(2))
        .await
        .context("second agent should register")?;

    first_agent.start_kill()?;
    first_agent.wait().await?;

    second_agent.start_kill()?;
    second_agent.wait().await?;

    cluster.shutdown().await?;

    Ok(())
}
