use paddler_messaging::balancer_desired_state::BalancerDesiredState;

use crate::state_database_error::StateDatabaseError;
use crate::state_database_file::StateDatabaseFile;
use crate::state_database_memory::StateDatabaseMemory;
use crate::state_database_type::StateDatabaseType;
use crate::stored_balancer_desired_state::StoredBalancerDesiredState;

pub enum StateDatabase {
    File(StateDatabaseFile),
    Memory(StateDatabaseMemory),
}

impl StateDatabase {
    #[must_use]
    pub const fn in_memory(stored_balancer_desired_state: StoredBalancerDesiredState) -> Self {
        Self::Memory(StateDatabaseMemory::new(stored_balancer_desired_state))
    }

    /// # Errors
    ///
    /// Returns [`StateDatabaseError::FileReadFailed`] when the state database file exists but cannot be read, [`StateDatabaseError::FileContentsInvalid`] when its contents are not a valid state database, and [`StateDatabaseError::SchemaVersionUnsupported`] when it uses a schema version this Paddler does not read.
    pub async fn read_balancer_desired_state(
        &self,
    ) -> Result<StoredBalancerDesiredState, StateDatabaseError> {
        match self {
            Self::File(file) => file.read_balancer_desired_state().await,
            Self::Memory(memory) => Ok(memory.read_balancer_desired_state()),
        }
    }

    /// # Errors
    ///
    /// Returns [`StateDatabaseError::FileHasNoParentDirectory`] when the state database file has no parent directory, [`StateDatabaseError::FilePermissionsUnreadable`] when the access mode of the existing file cannot be read, [`StateDatabaseError::FileReplacementCreationFailed`] when the replacement file cannot be created, [`StateDatabaseError::FileWriteFailed`] when the state cannot be written into it, [`StateDatabaseError::FileSyncFailed`] when it cannot be synced, and [`StateDatabaseError::FileReplaceFailed`] when it cannot replace the state database file.
    pub async fn store_balancer_desired_state(
        &self,
        balancer_desired_state: &BalancerDesiredState,
    ) -> Result<(), StateDatabaseError> {
        match self {
            Self::File(file) => {
                file.store_balancer_desired_state(balancer_desired_state)
                    .await
            }
            Self::Memory(memory) => {
                memory.store_balancer_desired_state(balancer_desired_state);

                Ok(())
            }
        }
    }
}

impl From<StateDatabaseType> for StateDatabase {
    fn from(state_database_type: StateDatabaseType) -> Self {
        match state_database_type {
            StateDatabaseType::File(path) => Self::File(StateDatabaseFile::new(path)),
            StateDatabaseType::Memory(stored_balancer_desired_state) => {
                Self::in_memory(stored_balancer_desired_state)
            }
        }
    }
}
