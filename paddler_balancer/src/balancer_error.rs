use thiserror::Error;

use paddler_state_database::state_database_error::StateDatabaseError;

#[derive(Debug, Error)]
pub enum DesiredStateReplacementError {
    #[error(transparent)]
    StateDatabase(#[from] StateDatabaseError),
    #[error(
        "The balancer desired state at version {version} cannot be replaced, because no later version is representable"
    )]
    VersionsExhausted { version: u64 },
}
