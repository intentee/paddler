use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use paddler_cluster::cluster::Cluster;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_tests::in_process_cluster_backend::InProcessClusterBackend;
use tokio::time::timeout;

const SHUTDOWN_BUDGET: Duration = Duration::from_secs(5);

#[tokio::test(flavor = "multi_thread")]
async fn single_agent_cluster_shutdown_completes_within_five_seconds() -> Result<()> {
    let cluster = Cluster::start(
        &InProcessClusterBackend::default(),
        ClusterParams {
            wait_for_slots_ready: false,
            ..ClusterParams::default()
        },
    )
    .await?;

    assert_eq!(cluster.agents.len(), 1);

    timeout(SHUTDOWN_BUDGET, cluster.shutdown())
        .await
        .with_context(|| {
            format!(
                "single-agent in-process cluster shutdown did not complete within {SHUTDOWN_BUDGET:?}; \
                 this matches the user-observed full-suite hang and indicates a resource leak or \
                 a stuck background task"
            )
        })??;

    Ok(())
}
