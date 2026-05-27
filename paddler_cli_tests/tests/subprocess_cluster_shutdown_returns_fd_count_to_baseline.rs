#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    any(target_os = "macos", target_os = "linux")
))]

use anyhow::Result;
use paddler_cli_tests::resource_snapshot::ResourceSnapshot;
use paddler_cli_tests::subprocess_cluster_lifecycle_in_dedicated_runtime::subprocess_cluster_lifecycle_in_dedicated_runtime;

#[test]
fn subprocess_cluster_shutdown_returns_fd_count_to_baseline() -> Result<()> {
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
