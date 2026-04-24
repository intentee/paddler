use anyhow::Result;
use paddler_tests::subprocess_cluster::SubprocessCluster;
use paddler_tests::subprocess_cluster_params::SubprocessClusterParams;

#[tokio::test(flavor = "multi_thread")]
async fn empty_subprocess_cluster_starts_and_exits_after_sigterm() -> Result<()> {
    let cluster = SubprocessCluster::start(SubprocessClusterParams {
        agent_count: 0,
        wait_for_slots_ready: false,
        ..SubprocessClusterParams::default()
    })
    .await?;

    cluster.shutdown().await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn single_subprocess_agent_registers_and_exits_after_sigterm() -> Result<()> {
    let cluster = SubprocessCluster::start(SubprocessClusterParams {
        agent_count: 1,
        wait_for_slots_ready: false,
        ..SubprocessClusterParams::default()
    })
    .await?;

    assert_eq!(cluster.agent_ids.len(), 1);

    cluster.shutdown().await?;

    Ok(())
}
