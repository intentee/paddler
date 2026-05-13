#![cfg(feature = "tests_that_use_compiled_paddler")]

use anyhow::Result;
use paddler_tests::resource_snapshot::ResourceSnapshot;
use paddler_tests::subprocess_cluster_lifecycle_in_dedicated_runtime::subprocess_cluster_lifecycle_in_dedicated_runtime;

#[test]
fn subprocess_cluster_shutdown_returns_fd_count_to_baseline() -> Result<()> {
    // Amortize per-process one-time init triggered by exercising the
    // subprocess-cluster machinery — `LazyLock` for the paddler binary path,
    // the logger, root cert stores, and any OS-level resources kqueue/signalfd
    // opens lazily — so the measured snapshot bracket observes steady state.
    // Each lifecycle owns its tokio runtime end-to-end; dropping the runtime
    // synchronously closes its I/O driver and worker threads, so a leftover fd
    // after the drop is a genuine cluster-side leak.
    subprocess_cluster_lifecycle_in_dedicated_runtime()?;

    let before = ResourceSnapshot::try_from_self()?;

    subprocess_cluster_lifecycle_in_dedicated_runtime()?;

    let after = ResourceSnapshot::try_from_self()?;
    let diff = after.diff(&before);

    assert_eq!(
        diff.open_file_descriptors_grew_by,
        0,
        "subprocess cluster lifecycle leaked file descriptors across a complete tokio runtime lifecycle: {summary}",
        summary = diff.pretty_summary(),
    );

    Ok(())
}
