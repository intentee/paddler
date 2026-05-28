
use anyhow::Result;
use paddler_cli_tests::agent_config::AgentConfig;
use paddler_cli_tests::start_subprocess_cluster::start_subprocess_cluster;
use paddler_cli_tests::subprocess_cluster_params::SubprocessClusterParams;

#[tokio::test(flavor = "multi_thread")]
async fn empty_subprocess_cluster_starts_and_exits_after_sigterm() -> Result<()> {
    let cluster = start_subprocess_cluster(SubprocessClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..SubprocessClusterParams::default()
    })
    .await?;

    cluster.shutdown().await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn single_subprocess_agent_registers_and_exits_after_sigterm() -> Result<()> {
    let cluster = start_subprocess_cluster(SubprocessClusterParams {
        agents: AgentConfig::uniform(1, 4),
        wait_for_slots_ready: false,
        ..SubprocessClusterParams::default()
    })
    .await?;

    assert_eq!(cluster.agent_ids.len(), 1);

    cluster.shutdown().await?;

    Ok(())
}
