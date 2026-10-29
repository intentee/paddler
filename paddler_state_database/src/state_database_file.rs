use std::fs::File;
use std::fs::Permissions;
use std::fs::metadata;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::from_str;
use serde_json::from_value;
use serde_json::to_writer_pretty;
use tempfile::NamedTempFile;
use tempfile::PersistError;
use tokio::fs::read_to_string;
use tokio::sync::Mutex;

use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_spawn_resuming_panic::spawn_blocking_resuming_panic::spawn_blocking_resuming_panic;

use crate::schema::Schema;
use crate::schema_version_header::SchemaVersionHeader;
use crate::state_database_error::StateDatabaseError;
use crate::state_database_schema_version::StateDatabaseSchemaVersion;
use crate::stored_balancer_desired_state::StoredBalancerDesiredState;

const NEW_STATE_FILE_ACCESS_MODE: u32 = 0o600;

fn replacement_permissions(state_path: &Path) -> Result<Permissions, StateDatabaseError> {
    match metadata(state_path) {
        Ok(state_metadata) => Ok(state_metadata.permissions()),
        Err(missing_state) if missing_state.kind() == ErrorKind::NotFound => {
            Ok(Permissions::from_mode(NEW_STATE_FILE_ACCESS_MODE))
        }
        Err(source) => Err(StateDatabaseError::FilePermissionsUnreadable {
            path: state_path.to_path_buf(),
            source,
        }),
    }
}

fn write_replacement(
    replacement: &File,
    replacement_path: &Path,
    schema: &Schema,
) -> Result<(), StateDatabaseError> {
    to_writer_pretty(replacement, schema).map_err(|source| {
        StateDatabaseError::FileWriteFailed {
            path: replacement_path.to_path_buf(),
            source,
        }
    })?;

    replacement
        .sync_all()
        .map_err(|source| StateDatabaseError::FileSyncFailed {
            path: replacement_path.to_path_buf(),
            source,
        })
}

fn replace_state_file(state_path: &Path, schema: &Schema) -> Result<(), StateDatabaseError> {
    let state_directory =
        state_path
            .parent()
            .ok_or_else(|| StateDatabaseError::FileHasNoParentDirectory {
                path: state_path.to_path_buf(),
            })?;
    let permissions = replacement_permissions(state_path)?;
    let replacement = NamedTempFile::new_in(state_directory)
        .and_then(|replacement| {
            replacement
                .as_file()
                .set_permissions(permissions)
                .map(|()| replacement)
        })
        .map_err(|source| StateDatabaseError::FileReplacementCreationFailed {
            directory: state_directory.to_path_buf(),
            source,
        })?;
    let replacement_path = replacement.path().to_path_buf();

    write_replacement(replacement.as_file(), &replacement_path, schema).and_then(|()| {
        replacement
            .persist(state_path)
            .map(drop)
            .map_err(
                |PersistError { error, .. }| StateDatabaseError::FileReplaceFailed {
                    path: state_path.to_path_buf(),
                    replacement_path,
                    source: error,
                },
            )
    })
}

pub struct StateDatabaseFile {
    path: PathBuf,
    store_lock: Arc<Mutex<()>>,
}

impl StateDatabaseFile {
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            store_lock: Arc::new(Mutex::new(())),
        }
    }

    fn parse_schema(&self, content: &str) -> Result<Schema, StateDatabaseError> {
        let SchemaVersionHeader { version } =
            from_str(content).map_err(|source| StateDatabaseError::FileContentsInvalid {
                path: self.path.clone(),
                source,
            })?;

        from_value::<StateDatabaseSchemaVersion>(version.clone()).map_err(|source| {
            StateDatabaseError::SchemaVersionUnsupported {
                found: version.to_string(),
                path: self.path.clone(),
                source,
            }
        })?;

        from_str(content).map_err(|source| StateDatabaseError::FileContentsInvalid {
            path: self.path.clone(),
            source,
        })
    }

    pub async fn read_balancer_desired_state(
        &self,
    ) -> Result<StoredBalancerDesiredState, StateDatabaseError> {
        match read_to_string(&self.path).await {
            Ok(content) => Ok(StoredBalancerDesiredState::Stored(Box::new(
                self.parse_schema(&content)?.balancer_desired_state,
            ))),
            Err(read_error) if read_error.kind() == ErrorKind::NotFound => {
                Ok(StoredBalancerDesiredState::NothingStored)
            }
            Err(source) => Err(StateDatabaseError::FileReadFailed {
                path: self.path.clone(),
                source,
            }),
        }
    }

    pub async fn store_balancer_desired_state(
        &self,
        balancer_desired_state: &BalancerDesiredState,
    ) -> Result<(), StateDatabaseError> {
        let schema = Schema {
            balancer_desired_state: balancer_desired_state.clone(),
            version: StateDatabaseSchemaVersion::default(),
        };
        let store_guard = self.store_lock.clone().lock_owned().await;
        let state_path = self.path.clone();

        spawn_blocking_resuming_panic(move || {
            let replacement_outcome = replace_state_file(&state_path, &schema);

            drop(store_guard);

            replacement_outcome
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error as _;
    #[cfg(target_os = "linux")]
    use std::fs::OpenOptions;
    use std::fs::Permissions;
    use std::io::ErrorKind;
    use std::os::unix::fs::PermissionsExt as _;
    #[cfg(target_os = "linux")]
    use std::os::unix::fs::symlink;
    use std::path::Path;
    use std::path::PathBuf;

    use tempfile::NamedTempFile;
    use tempfile::TempDir;
    use tokio::fs::metadata;
    use tokio::fs::read_to_string;
    use tokio::fs::set_permissions;
    use tokio::fs::write;

    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;

    use super::StateDatabaseFile;
    #[cfg(target_os = "linux")]
    use super::write_replacement;
    use crate::schema::Schema;
    use crate::state_database_error::StateDatabaseError;
    use crate::state_database_schema_version::StateDatabaseSchemaVersion;
    use crate::stored_balancer_desired_state::StoredBalancerDesiredState;

    fn state_with_model(model_uri: &str) -> BalancerDesiredState {
        BalancerDesiredState {
            model: AgentDesiredModel::Uri(model_uri.to_owned()),
            ..BalancerDesiredState::default()
        }
    }

    #[tokio::test]
    async fn reads_back_the_stored_desired_state_from_the_file() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("state.json");
        let database = StateDatabaseFile::new(path.clone());
        let desired_state = state_with_model("stored_model_uri");

        database
            .store_balancer_desired_state(&desired_state)
            .await
            .unwrap();

        assert_eq!(
            database.read_balancer_desired_state().await.unwrap(),
            StoredBalancerDesiredState::Stored(Box::new(desired_state))
        );
        assert!(metadata(&path).await.unwrap().is_file());
    }

    #[tokio::test]
    async fn reading_a_missing_file_reports_nothing_stored_and_creates_no_file() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("not_yet_created.json");

        assert_eq!(
            StateDatabaseFile::new(path.clone())
                .read_balancer_desired_state()
                .await
                .unwrap(),
            StoredBalancerDesiredState::NothingStored
        );
        assert!(
            metadata(&path)
                .await
                .is_err_and(|missing| missing.kind() == ErrorKind::NotFound)
        );
    }

    #[tokio::test]
    async fn reading_invalid_json_returns_parse_error() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();

        write(&path, b"this is not valid json").await.unwrap();

        assert!(matches!(
            StateDatabaseFile::new(path.clone()).read_balancer_desired_state().await,
            Err(StateDatabaseError::FileContentsInvalid { path: invalid_path, .. }) if invalid_path == path
        ));
    }

    #[tokio::test]
    async fn reading_an_empty_file_fails_and_leaves_the_file_untouched() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();

        assert!(matches!(
            StateDatabaseFile::new(path.clone()).read_balancer_desired_state().await,
            Err(StateDatabaseError::FileContentsInvalid { path: invalid_path, .. }) if invalid_path == path
        ));
        assert_eq!(read_to_string(&path).await.unwrap(), "");
    }

    #[tokio::test]
    async fn reading_a_directory_path_returns_non_not_found_error() {
        let temp_dir = TempDir::new().unwrap();

        assert!(matches!(
            StateDatabaseFile::new(temp_dir.path().to_path_buf()).read_balancer_desired_state().await,
            Err(StateDatabaseError::FileReadFailed { source, .. })
                if source.kind() == ErrorKind::IsADirectory
        ));
    }

    #[tokio::test]
    async fn storing_fails_when_the_parent_directory_is_missing() {
        let temp_dir = TempDir::new().unwrap();
        let missing_directory = temp_dir.path().join("missing_directory");
        let database = StateDatabaseFile::new(missing_directory.join("state.json"));

        assert!(matches!(
            database
                .store_balancer_desired_state(&BalancerDesiredState::default())
                .await,
            Err(StateDatabaseError::FileReplacementCreationFailed { directory, source })
                if directory == missing_directory && source.kind() == ErrorKind::NotFound
        ));
    }

    #[tokio::test]
    async fn storing_at_the_filesystem_root_has_no_directory_for_its_replacement() {
        assert!(matches!(
            StateDatabaseFile::new(PathBuf::from("/"))
                .store_balancer_desired_state(&BalancerDesiredState::default())
                .await,
            Err(StateDatabaseError::FileHasNoParentDirectory { path }) if path == Path::new("/")
        ));
    }

    #[tokio::test]
    async fn storing_under_a_regular_file_cannot_read_the_access_mode_of_its_state() {
        let regular_file = NamedTempFile::new().unwrap();
        let path = regular_file.path().join("state.json");

        assert!(matches!(
            StateDatabaseFile::new(path.clone())
                .store_balancer_desired_state(&BalancerDesiredState::default())
                .await,
            Err(StateDatabaseError::FilePermissionsUnreadable { path: unreadable_path, source })
                if unreadable_path == path && source.kind() == ErrorKind::NotADirectory
        ));
    }

    #[tokio::test]
    async fn storing_fails_when_a_directory_occupies_the_state_path() {
        let temp_dir = TempDir::new().unwrap();
        let database = StateDatabaseFile::new(temp_dir.path().to_path_buf());

        assert!(matches!(
            database
                .store_balancer_desired_state(&BalancerDesiredState::default())
                .await,
            Err(StateDatabaseError::FileReplaceFailed { path, .. }) if path == temp_dir.path()
        ));
    }

    #[tokio::test]
    async fn a_store_that_fails_leaves_the_previous_state_readable() {
        let temp_dir = TempDir::new().unwrap();
        let database = StateDatabaseFile::new(temp_dir.path().join("state.json"));
        let previous_state = state_with_model("previous_model_uri");

        database
            .store_balancer_desired_state(&previous_state)
            .await
            .unwrap();
        set_permissions(temp_dir.path(), Permissions::from_mode(0o500))
            .await
            .unwrap();

        let failed_store = database
            .store_balancer_desired_state(&state_with_model("next_model_uri"))
            .await;
        let state_after_the_failed_store = database.read_balancer_desired_state().await.unwrap();

        set_permissions(temp_dir.path(), Permissions::from_mode(0o700))
            .await
            .unwrap();

        assert!(matches!(
            failed_store,
            Err(StateDatabaseError::FileReplacementCreationFailed { source, .. })
                if source.kind() == ErrorKind::PermissionDenied
        ));
        assert_eq!(
            state_after_the_failed_store,
            StoredBalancerDesiredState::Stored(Box::new(previous_state))
        );
    }

    #[cfg(target_os = "linux")]
    fn write_replacement_to_device(device_path: &Path) -> Result<(), StateDatabaseError> {
        write_replacement(
            &OpenOptions::new().write(true).open(device_path).unwrap(),
            device_path,
            &Schema {
                balancer_desired_state: BalancerDesiredState::default(),
                version: StateDatabaseSchemaVersion::default(),
            },
        )
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_replacement_on_a_device_that_refuses_to_sync_fails_to_sync() {
        assert!(matches!(
            write_replacement_to_device(Path::new("/dev/null")),
            Err(StateDatabaseError::FileSyncFailed { path, source })
                if path == Path::new("/dev/null") && source.kind() == ErrorKind::InvalidInput
        ));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_replacement_on_a_full_device_fails_to_write_its_schema() {
        assert!(matches!(
            write_replacement_to_device(Path::new("/dev/full")),
            Err(StateDatabaseError::FileWriteFailed { path, source })
                if path == Path::new("/dev/full")
                    && source.io_error_kind() == Some(ErrorKind::StorageFull)
        ));
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn storing_never_writes_through_a_link_planted_at_the_old_replacement_path() {
        let temp_dir = TempDir::new().unwrap();
        let unrelated_file = temp_dir.path().join("unrelated.txt");

        write(&unrelated_file, "unrelated contents").await.unwrap();
        symlink(&unrelated_file, temp_dir.path().join("state.json.next")).unwrap();

        StateDatabaseFile::new(temp_dir.path().join("state.json"))
            .store_balancer_desired_state(&state_with_model("stored_model_uri"))
            .await
            .unwrap();

        assert_eq!(
            read_to_string(&unrelated_file).await.unwrap(),
            "unrelated contents"
        );
    }

    #[tokio::test]
    async fn storing_keeps_the_access_mode_of_the_replaced_state_file() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("state.json");
        let database = StateDatabaseFile::new(path.clone());

        database
            .store_balancer_desired_state(&state_with_model("previous_model_uri"))
            .await
            .unwrap();
        set_permissions(&path, Permissions::from_mode(0o640))
            .await
            .unwrap();
        database
            .store_balancer_desired_state(&state_with_model("next_model_uri"))
            .await
            .unwrap();

        assert_eq!(
            metadata(&path).await.unwrap().permissions().mode() & 0o777,
            0o640
        );
    }

    #[tokio::test]
    async fn a_new_state_file_is_readable_only_by_its_owner() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("state.json");

        StateDatabaseFile::new(path.clone())
            .store_balancer_desired_state(&state_with_model("stored_model_uri"))
            .await
            .unwrap();

        assert_eq!(
            metadata(&path).await.unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[tokio::test]
    async fn reading_a_file_of_an_older_schema_version_names_the_version_it_found() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();

        write(&path, br#"{"balancer_desired_state": {}, "version": "1"}"#)
            .await
            .unwrap();

        assert_eq!(
            StateDatabaseFile::new(path.clone())
                .read_balancer_desired_state()
                .await
                .err()
                .as_ref()
                .map(ToString::to_string),
            Some(format!(
                r#"The state database file '{}' uses schema version "1", but this version of Paddler only reads version 2. Start from a new state database file."#,
                path.display()
            ))
        );
    }

    #[tokio::test]
    async fn reading_a_file_of_an_older_schema_version_keeps_why_the_version_was_refused() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();

        write(&path, br#"{"balancer_desired_state": {}, "version": "1"}"#)
            .await
            .unwrap();

        let refusal = StateDatabaseFile::new(path)
            .read_balancer_desired_state()
            .await
            .unwrap_err();

        assert_eq!(
            refusal.source().map(ToString::to_string),
            Some("unknown variant `1`, expected `2`".to_owned())
        );
    }
}
