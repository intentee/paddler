use std::fs;

use paddler_balancer::state_database_type::StateDatabaseType;
use paddler_bootstrap::balancer_runner::BalancerRunner;
use paddler_bootstrap::bootstrap_error::BootstrapError;
use tempfile::NamedTempFile;
use tokio_util::sync::CancellationToken;

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

    params.state_database_type =
        StateDatabaseType::File(corrupt_state_database.path().to_path_buf());

    let start_error = BalancerRunner::start(params)
        .await
        .err()
        .expect("a runner must not start from a corrupt state database");

    assert!(matches!(
        start_error,
        BootstrapError::StateDatabaseReadFailed { source }
            if source.root_cause().downcast_ref::<serde_json::Error>().is_some()
    ));
}
