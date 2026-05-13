#![cfg(feature = "tests_that_use_compiled_paddler")]

use std::time::Duration;
use std::time::Instant;

use anyhow::Result;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::resource_snapshot::ResourceSnapshot;
use paddler_tests::start_subprocess_cluster::start_subprocess_cluster;
use paddler_tests::subprocess_cluster_params::SubprocessClusterParams;

const LIFECYCLE_COUNT: usize = 20;

/// Same justification as the in-process 100-iteration test: small bounded growth
/// for process-level init (`LazyLock` for `PADDLER_BINARY_PATH`, logger).
/// Subprocess clusters do not open additional long-lived parent-side fds beyond
/// what is required to wait on each `Child`, and those must close on `wait()`.
const ALLOWED_GROWTH_FOR_PROCESS_LEVEL_INIT: usize = 4;

/// In isolation a subprocess cluster lifecycle measures around 1 s; under
/// contention it can be slower but a single lifecycle exceeding 10 s is a clear
/// stall signal (port-bind contention or stuck child wait).
const PER_LIFECYCLE_BUDGET: Duration = Duration::from_secs(10);

#[tokio::test(flavor = "multi_thread")]
async fn subprocess_cluster_lifecycle_repeated_20_times_does_not_grow_fd_count() -> Result<()> {
    let before = ResourceSnapshot::try_from_self()?;

    let mut slowest_lifecycle_index: usize = 0;
    let mut slowest_lifecycle_duration = Duration::ZERO;

    for lifecycle_index in 0..LIFECYCLE_COUNT {
        let started_at = Instant::now();

        let cluster = start_subprocess_cluster(SubprocessClusterParams {
            agents: AgentConfig::uniform(1, 4),
            wait_for_slots_ready: false,
            ..SubprocessClusterParams::default()
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
        "subprocess cluster lifecycle leaked file descriptors over {LIFECYCLE_COUNT} iterations: {summary} (allowed growth for process-level init: {ALLOWED_GROWTH_FOR_PROCESS_LEVEL_INIT})",
        summary = diff.pretty_summary(),
    );

    assert!(
        slowest_lifecycle_duration <= PER_LIFECYCLE_BUDGET,
        "subprocess cluster lifecycle iteration {slowest_lifecycle_index} took {slowest_lifecycle_duration:?}, exceeding the per-lifecycle budget of {PER_LIFECYCLE_BUDGET:?}; this indicates resource contention, a stuck child wait, or port-bind backoff"
    );

    Ok(())
}
