use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use paddler_tests::in_process_cluster_params::InProcessClusterParams;
use paddler_tests::start_in_process_cluster::start_in_process_cluster;
use tokio::time::timeout;

const SHUTDOWN_BUDGET: Duration = Duration::from_secs(5);

/// Mirrors the exact scenario that hangs during the full integration suite:
/// `single_agent_registers_and_shuts_down_without_timeout` in
/// `harness_in_process_cluster_shutdown.rs` runs in ~0.9 s in isolation but
/// blocks past 60 s under sustained suite load. If `cluster.shutdown()` for
/// this configuration ever fails to land inside `SHUTDOWN_BUDGET`, this test
/// fails inside the test process before cargo's heartbeat fires.
#[tokio::test(flavor = "multi_thread")]
async fn single_agent_in_process_cluster_shutdown_completes_within_five_seconds() -> Result<()> {
    let cluster = start_in_process_cluster(InProcessClusterParams {
        wait_for_slots_ready: false,
        ..InProcessClusterParams::default()
    })
    .await?;

    assert_eq!(cluster.agent_ids.len(), 1);

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
