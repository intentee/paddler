use std::fs;

use tempfile::NamedTempFile;
use tokio_util::sync::CancellationToken;

use paddler_balancer_runner::balancer_runner::BalancerRunner;
use paddler_balancer_runner::balancer_runner_error::BalancerRunnerError;
use paddler_state_database::state_database_error::StateDatabaseError;
use paddler_state_database::state_database_type::StateDatabaseType;

use crate::ephemeral_balancer_runner_params::ephemeral_balancer_runner_params;

#[tokio::test]
async fn balancer_runner_fails_to_start_when_state_database_file_is_corrupt() {
    let corrupt_state_database =
        NamedTempFile::new().expect("a temporary state database file must be creatable");

    fs::write(
        corrupt_state_database.path(),
        b"this is not a valid state database",
    )
    .expect("the corrupt state database must be writable");

    let mut params = ephemeral_balancer_runner_params(CancellationToken::new());

    params.runner_config.state_database_type =
        StateDatabaseType::File(corrupt_state_database.path().to_path_buf());

    let start_error = BalancerRunner::start(params)
        .await
        .err()
        .expect("a runner must not start from a corrupt state database");

    assert!(matches!(
        start_error,
        BalancerRunnerError::StateDatabaseReadFailed {
            source: StateDatabaseError::FileContentsInvalid { path, .. }
        } if path == corrupt_state_database.path()
    ));
}
