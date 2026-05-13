use std::time::Duration;
use std::time::Instant;

use anyhow::Result;
use paddler_tests::in_process_cluster_params::InProcessClusterParams;
use paddler_tests::resource_snapshot::ResourceSnapshot;
use paddler_tests::start_in_process_cluster::start_in_process_cluster;

const LIFECYCLE_COUNT: usize = 100;

/// `LazyLock` and logger initialization at the cargo-test process level open a
/// small, bounded set of files exactly once during the first lifecycle. Anything
/// over this tolerance after `LIFECYCLE_COUNT` iterations is a real per-lifecycle
/// leak, because a real leak compounds with iteration count and this tolerance
/// does not.
const ALLOWED_GROWTH_FOR_PROCESS_LEVEL_INIT: usize = 4;

/// A single in-process cluster lifecycle measured at ~0.9 s in isolation
/// (`in_process_cluster_shutdown_completes_within_five_seconds`). Any lifecycle
/// that exceeds 3 s is either resource-contended or stuck on shutdown.
const PER_LIFECYCLE_BUDGET: Duration = Duration::from_secs(3);

#[tokio::test(flavor = "multi_thread")]
async fn in_process_cluster_lifecycle_repeated_100_times_does_not_grow_fd_count() -> Result<()> {
    let before = ResourceSnapshot::try_from_self()?;

    let mut slowest_lifecycle_index: usize = 0;
    let mut slowest_lifecycle_duration = Duration::ZERO;

    for lifecycle_index in 0..LIFECYCLE_COUNT {
        let started_at = Instant::now();

        let cluster = start_in_process_cluster(InProcessClusterParams {
            wait_for_slots_ready: false,
            ..InProcessClusterParams::default()
        })
        .await?;
        cluster.shutdown().await?;

        let elapsed = started_at.elapsed();
        if elapsed > slowest_lifecycle_duration {
            slowest_lifecycle_duration = elapsed;
            slowest_lifecycle_index = lifecycle_index;
        }
    }

    let after = ResourceSnapshot::try_from_self()?;
    let diff = after.diff(&before);

    assert!(
        diff.open_file_descriptors_grew_by <= ALLOWED_GROWTH_FOR_PROCESS_LEVEL_INIT,
        "in-process cluster lifecycle leaked file descriptors over {LIFECYCLE_COUNT} iterations: {summary} (allowed growth for process-level init: {ALLOWED_GROWTH_FOR_PROCESS_LEVEL_INIT})",
        summary = diff.pretty_summary(),
    );

    assert!(
        slowest_lifecycle_duration <= PER_LIFECYCLE_BUDGET,
        "in-process cluster lifecycle iteration {slowest_lifecycle_index} took {slowest_lifecycle_duration:?}, exceeding the per-lifecycle budget of {PER_LIFECYCLE_BUDGET:?}; this indicates resource contention or accumulating slowdown over the {LIFECYCLE_COUNT}-iteration loop"
    );

    Ok(())
}
