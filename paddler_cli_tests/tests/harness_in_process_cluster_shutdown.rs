use anyhow::Result;
use paddler_cli_tests::in_process_cluster_params::InProcessClusterParams;
use paddler_cli_tests::start_in_process_cluster::start_in_process_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn empty_cluster_starts_and_shuts_down_without_timeout() -> Result<()> {
    let cluster = start_in_process_cluster(InProcessClusterParams {
        agent: None,
        wait_for_slots_ready: false,
        ..InProcessClusterParams::default()
    })
    .await?;

    cluster.shutdown().await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn single_agent_registers_and_shuts_down_without_timeout() -> Result<()> {
    let cluster = start_in_process_cluster(InProcessClusterParams {
        wait_for_slots_ready: false,
        ..InProcessClusterParams::default()
    })
    .await?;

    assert_eq!(cluster.agent_ids.len(), 1);

    cluster.shutdown().await?;

    Ok(())
}
