use anyhow::Result;
use paddler_tests::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn empty_cluster_starts_and_shuts_down_without_timeout() -> Result<()> {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await?;

    cluster.shutdown().await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn single_agent_registers_and_shuts_down_without_timeout() -> Result<()> {
    let cluster = start_cluster(ClusterParams {
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await?;

    assert_eq!(cluster.agent_ids.len(), 1);

    cluster.shutdown().await?;

    Ok(())
}
