use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;
use tokio::time::timeout;

const SHUTDOWN_BUDGET: Duration = Duration::from_secs(5);

#[tokio::test(flavor = "multi_thread")]
async fn cluster_shutdown_completes_within_five_seconds() -> Result<()> {
    let cluster = start_cluster(ClusterParams {
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await?;

    timeout(SHUTDOWN_BUDGET, cluster.shutdown())
        .await
        .with_context(|| {
            format!(
                "in-process cluster shutdown did not complete within {SHUTDOWN_BUDGET:?}; \
                 this indicates a resource leak or a stuck background task that survives shutdown"
            )
        })??;

    Ok(())
}
