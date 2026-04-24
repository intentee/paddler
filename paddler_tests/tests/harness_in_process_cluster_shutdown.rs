use anyhow::Result;
use paddler_tests::in_process_cluster::InProcessCluster;
use paddler_tests::in_process_cluster_params::InProcessClusterParams;

#[tokio::test(flavor = "multi_thread")]
async fn empty_cluster_starts_and_shuts_down_without_timeout() -> Result<()> {
    let cluster = InProcessCluster::start(InProcessClusterParams {
        agent_count: 0,
        wait_for_slots_ready: false,
        ..InProcessClusterParams::default()
    })
    .await?;

    cluster.shutdown().await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn single_agent_registers_and_shuts_down_without_timeout() -> Result<()> {
    let cluster = InProcessCluster::start(InProcessClusterParams {
        agent_count: 1,
        wait_for_slots_ready: false,
        ..InProcessClusterParams::default()
    })
    .await?;

    assert_eq!(cluster.agent_ids.len(), 1);

    cluster.shutdown().await?;

    Ok(())
}
