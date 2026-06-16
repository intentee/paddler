#![cfg(any(target_os = "macos", target_os = "linux"))]

use anyhow::Result;
use paddler_cluster::cluster::Cluster;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_tests::in_process_cluster_backend::InProcessClusterBackend;
use paddler_tests::resource_snapshot::ResourceSnapshot;

#[tokio::test(flavor = "multi_thread")]
async fn cluster_shutdown_returns_fd_count_to_baseline() -> Result<()> {
    let before = ResourceSnapshot::try_from_self().await?;

    let cluster = Cluster::start(
        &InProcessClusterBackend::default(),
        ClusterParams {
            wait_for_slots_ready: false,
            ..ClusterParams::default()
        },
    )
    .await?;
    cluster.shutdown().await?;

    let after = ResourceSnapshot::try_from_self().await?;
    let diff = after.diff(&before);

    assert_eq!(
        diff.open_file_descriptors_grew_by,
        0,
        "in-process cluster lifecycle leaked file descriptors: {summary}",
        summary = diff.pretty_summary(),
    );

    Ok(())
}
