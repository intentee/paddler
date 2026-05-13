use anyhow::Result;
use paddler_tests::in_process_cluster_params::InProcessClusterParams;
use paddler_tests::resource_snapshot::ResourceSnapshot;
use paddler_tests::start_in_process_cluster::start_in_process_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn in_process_cluster_shutdown_returns_fd_count_to_baseline() -> Result<()> {
    let before = ResourceSnapshot::try_from_self()?;

    let cluster = start_in_process_cluster(InProcessClusterParams {
        wait_for_slots_ready: false,
        ..InProcessClusterParams::default()
    })
    .await?;
    cluster.shutdown().await?;

    let after = ResourceSnapshot::try_from_self()?;
    let diff = after.diff(&before);

    assert_eq!(
        diff.open_file_descriptors_grew_by,
        0,
        "in-process cluster lifecycle leaked file descriptors: {summary}",
        summary = diff.pretty_summary(),
    );

    Ok(())
}
