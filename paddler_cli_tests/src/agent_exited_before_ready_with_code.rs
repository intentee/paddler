use anyhow::Error;

use paddler_test_cluster_harness::cluster_harness_error::ClusterHarnessError;
use paddler_test_cluster_harness::process_end::ProcessEnd;

use crate::subprocess_cluster_error::SubprocessClusterError;

#[must_use]
pub fn agent_exited_before_ready_with_code(error: &Error, expected_exit_code: i32) -> bool {
    matches!(
        error.downcast_ref::<ClusterHarnessError>(),
        Some(ClusterHarnessError::AgentExitedBeforeReady {
            process_end: ProcessEnd::Failed(exit_failure),
            ..
        }) if matches!(
            exit_failure.downcast_ref::<SubprocessClusterError>(),
            Some(SubprocessClusterError::ProcessExitedWithFailure { exit_status })
                if exit_status.code() == Some(expected_exit_code)
        )
    )
}
