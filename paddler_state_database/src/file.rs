use std::ffi::OsString;
use std::io::ErrorKind;
use std::path::PathBuf;

use async_trait::async_trait;
use serde_json::from_str;
use serde_json::from_value;
use serde_json::to_string_pretty;
use tokio::fs::File as TokioFile;
use tokio::fs::read_to_string;
use tokio::fs::rename;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

use paddler_messaging::balancer_desired_state::BalancerDesiredState;

use crate::schema::Schema;
use crate::schema_version_header::SchemaVersionHeader;
use crate::state_database::StateDatabase;
use crate::state_database_error::StateDatabaseError;
use crate::state_database_schema_version::StateDatabaseSchemaVersion;
use crate::stored_balancer_desired_state::StoredBalancerDesiredState;

const REPLACEMENT_SUFFIX: &str = ".next";

pub struct File {
    path: PathBuf,
    store_lock: Mutex<()>,
}

impl File {
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            store_lock: Mutex::new(()),
        }
    }

    fn replacement_path(&self) -> PathBuf {
        let mut replacement_path = OsString::from(self.path.as_os_str());

        replacement_path.push(REPLACEMENT_SUFFIX);

        PathBuf::from(replacement_path)
    }

    fn parse_schema(&self, content: &str) -> Result<Schema, StateDatabaseError> {
        let SchemaVersionHeader { version } =
            from_str(content).map_err(|source| StateDatabaseError::FileContentsInvalid {
                path: self.path.clone(),
                source,
            })?;

        if from_value::<StateDatabaseSchemaVersion>(version.clone()).is_err() {
            return Err(StateDatabaseError::SchemaVersionUnsupported {
                found: version.to_string(),
                path: self.path.clone(),
            });
        }

        from_str(content).map_err(|source| StateDatabaseError::FileContentsInvalid {
            path: self.path.clone(),
            source,
        })
    }

    async fn write_replacement(
        &self,
        replacement_path: &PathBuf,
        serialized_schema: &str,
    ) -> Result<(), StateDatabaseError> {
        let mut replacement = TokioFile::create(replacement_path)
            .await
            .map_err(|source| StateDatabaseError::FileCreationFailed {
                path: replacement_path.clone(),
                source,
            })?;

        replacement
            .write_all(serialized_schema.as_bytes())
            .await
            .map_err(|source| StateDatabaseError::FileWriteFailed {
                path: replacement_path.clone(),
                source,
            })?;

        replacement
            .sync_all()
            .await
            .map_err(|source| StateDatabaseError::FileSyncFailed {
                path: replacement_path.clone(),
                source,
            })
    }
}

#[async_trait]
impl StateDatabase for File {
    async fn read_balancer_desired_state(
        &self,
    ) -> Result<StoredBalancerDesiredState, StateDatabaseError> {
        match read_to_string(&self.path).await {
            Ok(content) => Ok(StoredBalancerDesiredState::Stored(
                self.parse_schema(&content)?.balancer_desired_state,
            )),
            Err(read_error) if read_error.kind() == ErrorKind::NotFound => {
                Ok(StoredBalancerDesiredState::NothingStored)
            }
            Err(source) => Err(StateDatabaseError::FileReadFailed {
                path: self.path.clone(),
                source,
            }),
        }
    }

    async fn store_balancer_desired_state(
        &self,
        balancer_desired_state: &BalancerDesiredState,
    ) -> Result<(), StateDatabaseError> {
        let serialized_schema = to_string_pretty(&Schema {
            balancer_desired_state: balancer_desired_state.clone(),
            version: StateDatabaseSchemaVersion::default(),
        })
        .map_err(StateDatabaseError::SchemaUnserializable)?;
        let replacement_path = self.replacement_path();
        let _store_guard = self.store_lock.lock().await;

        self.write_replacement(&replacement_path, &serialized_schema)
            .await?;

        rename(&replacement_path, &self.path)
            .await
            .map_err(|source| StateDatabaseError::FileReplaceFailed {
                path: self.path.clone(),
                replacement_path,
                source,
            })
    }
}

#[cfg(test)]
mod tests {
    use std::io::ErrorKind;
    #[cfg(target_os = "linux")]
    use std::os::unix::fs::symlink;

    use tempfile::NamedTempFile;
    use tempfile::TempDir;
    use tokio::fs::create_dir;
    use tokio::fs::metadata;
    use tokio::fs::read_to_string;
    use tokio::fs::write;

    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;
    use paddler_messaging::balancer_text_generation_settings::BalancerTextGenerationSettings;
    use paddler_messaging::chat_template::ChatTemplate;
    use paddler_messaging::chat_template_source::ChatTemplateSource;
    use paddler_messaging::decision_settings::DecisionSettings;
    use paddler_messaging::inference_mode::InferenceMode;

    use super::File;
    use crate::state_database::StateDatabase;
    use crate::state_database_error::StateDatabaseError;
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
        let database = File::new(path.clone());
        let desired_state = state_with_model("stored_model_uri");

        database
            .store_balancer_desired_state(&desired_state)
            .await
            .unwrap();

        assert_eq!(
            database.read_balancer_desired_state().await.unwrap(),
            StoredBalancerDesiredState::Stored(desired_state)
        );
        assert!(metadata(&path).await.unwrap().is_file());
    }

    #[tokio::test]
    async fn reading_a_missing_file_reports_nothing_stored_and_creates_no_file() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("not_yet_created.json");

        assert_eq!(
            File::new(path.clone())
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
            File::new(path.clone()).read_balancer_desired_state().await,
            Err(StateDatabaseError::FileContentsInvalid { path: invalid_path, .. }) if invalid_path == path
        ));
    }

    #[tokio::test]
    async fn reading_an_empty_file_fails_and_leaves_the_file_untouched() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();

        assert!(matches!(
            File::new(path.clone()).read_balancer_desired_state().await,
            Err(StateDatabaseError::FileContentsInvalid { path: invalid_path, .. }) if invalid_path == path
        ));
        assert_eq!(read_to_string(&path).await.unwrap(), "");
    }

    #[tokio::test]
    async fn reading_a_directory_path_returns_non_not_found_error() {
        let temp_dir = TempDir::new().unwrap();

        assert!(matches!(
            File::new(temp_dir.path().to_path_buf()).read_balancer_desired_state().await,
            Err(StateDatabaseError::FileReadFailed { source, .. })
                if source.kind() == ErrorKind::IsADirectory
        ));
    }

    #[tokio::test]
    async fn storing_fails_when_the_parent_directory_is_missing() {
        let temp_dir = TempDir::new().unwrap();
        let database = File::new(temp_dir.path().join("missing_directory").join("state.json"));
        let replacement_path = database.replacement_path();

        assert!(matches!(
            database
                .store_balancer_desired_state(&BalancerDesiredState::default())
                .await,
            Err(StateDatabaseError::FileCreationFailed { path, source })
                if path == replacement_path && source.kind() == ErrorKind::NotFound
        ));
    }

    #[tokio::test]
    async fn storing_fails_when_a_directory_occupies_the_state_path() {
        let temp_dir = TempDir::new().unwrap();
        let database = File::new(temp_dir.path().to_path_buf());

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
        let database = File::new(temp_dir.path().join("state.json"));
        let previous_state = state_with_model("previous_model_uri");

        database
            .store_balancer_desired_state(&previous_state)
            .await
            .unwrap();
        create_dir(database.replacement_path()).await.unwrap();

        assert!(
            database
                .store_balancer_desired_state(&state_with_model("next_model_uri"))
                .await
                .is_err()
        );
        assert_eq!(
            database.read_balancer_desired_state().await.unwrap(),
            StoredBalancerDesiredState::Stored(previous_state)
        );
    }

    #[cfg(target_os = "linux")]
    fn database_replacing_through_dev_full(temp_dir: &TempDir) -> File {
        let database = File::new(temp_dir.path().join("state.json"));

        symlink("/dev/full", database.replacement_path()).unwrap();

        database
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn storing_a_small_schema_through_a_character_device_fails_to_sync() {
        let temp_dir = TempDir::new().unwrap();

        assert!(matches!(
            database_replacing_through_dev_full(&temp_dir)
                .store_balancer_desired_state(&BalancerDesiredState::default())
                .await,
            Err(StateDatabaseError::FileSyncFailed { source, .. })
                if source.kind() == ErrorKind::InvalidInput
        ));
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn storing_a_large_schema_surfaces_the_write_error_during_write_all() {
        const TOKIO_FILE_BUFFER_BYTES: usize = 2 * 1024 * 1024;

        let temp_dir = TempDir::new().unwrap();

        assert!(matches!(
            database_replacing_through_dev_full(&temp_dir)
                .store_balancer_desired_state(&state_with_model(
                    &"x".repeat(TOKIO_FILE_BUFFER_BYTES * 2)
                ))
                .await,
            Err(StateDatabaseError::FileWriteFailed { source, .. })
                if source.kind() == ErrorKind::StorageFull
        ));
    }

    #[tokio::test]
    async fn persists_the_settings_of_inactive_inference_modes_across_instances() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("state.json");
        let desired_state = BalancerDesiredState {
            decision: DecisionSettings {
                pointer_head: AgentDesiredModel::Uri("pointer_head.gguf".to_owned()),
            },
            inference_mode: InferenceMode::Embeddings,
            text_generation: BalancerTextGenerationSettings {
                chat_template_source: ChatTemplateSource::Override(ChatTemplate {
                    content: "{% for message in messages %}{{ message.content }}{% endfor %}"
                        .to_owned(),
                }),
                ..BalancerTextGenerationSettings::default()
            },
            ..state_with_model("test_model_uri")
        };

        File::new(path.clone())
            .store_balancer_desired_state(&desired_state)
            .await
            .unwrap();

        assert_eq!(
            File::new(path).read_balancer_desired_state().await.unwrap(),
            StoredBalancerDesiredState::Stored(desired_state)
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
            File::new(path.clone())
                .read_balancer_desired_state()
                .await
                .err()
                .as_ref()
                .map(ToString::to_string),
            Some(
                StateDatabaseError::SchemaVersionUnsupported {
                    found: r#""1""#.to_owned(),
                    path,
                }
                .to_string()
            )
        );
    }
}
