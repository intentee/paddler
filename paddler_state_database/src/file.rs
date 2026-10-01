use std::io::ErrorKind;
use std::path::PathBuf;

use async_trait::async_trait;
use log::warn;
use serde_json::from_str;
use serde_json::to_string_pretty;
use tokio::fs::File as TokioFile;
use tokio::fs::read_to_string;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
use tokio::sync::watch;

use paddler_messaging::balancer_desired_state::BalancerDesiredState;

use crate::schema::Schema;
use crate::state_database::StateDatabase;
use crate::state_database_error::StateDatabaseError;

pub struct File {
    balancer_desired_state_notify_tx: watch::Sender<BalancerDesiredState>,
    path: PathBuf,
    write_lock: RwLock<()>,
}

impl File {
    #[must_use]
    pub fn new(
        balancer_desired_state_notify_tx: watch::Sender<BalancerDesiredState>,
        path: PathBuf,
    ) -> Self {
        Self {
            balancer_desired_state_notify_tx,
            path,
            write_lock: RwLock::new(()),
        }
    }

    async fn read_schema_from_file(&self) -> Result<Schema, StateDatabaseError> {
        match read_to_string(&self.path).await {
            Ok(content) => {
                from_str(&content).map_err(|source| StateDatabaseError::FileContentsInvalid {
                    path: self.path.clone(),
                    source,
                })
            }
            Err(read_error) if read_error.kind() == ErrorKind::NotFound => {
                warn!(
                    "State database file not found; trying to store the default state: '{}'",
                    self.path.display()
                );

                self.store_default_schema().await
            }
            Err(source) => Err(StateDatabaseError::FileReadFailed {
                path: self.path.clone(),
                source,
            }),
        }
    }

    async fn store_default_schema(&self) -> Result<Schema, StateDatabaseError> {
        let schema = Schema::default();

        self.store_schema(&schema).await?;

        Ok(schema)
    }

    async fn store_schema(&self, schema: &Schema) -> Result<(), StateDatabaseError> {
        let balancer_desired_state = schema.balancer_desired_state.clone();
        let _lock = self.write_lock.write().await;

        let serialized_schema =
            to_string_pretty(schema).map_err(StateDatabaseError::SchemaUnserializable)?;
        let mut file = TokioFile::create(&self.path).await.map_err(|source| {
            StateDatabaseError::FileCreationFailed {
                path: self.path.clone(),
                source,
            }
        })?;

        file.write_all(serialized_schema.as_bytes())
            .await
            .map_err(|source| StateDatabaseError::FileWriteFailed {
                path: self.path.clone(),
                source,
            })?;
        file.sync_all()
            .await
            .map_err(|source| StateDatabaseError::FileSyncFailed {
                path: self.path.clone(),
                source,
            })?;

        self.balancer_desired_state_notify_tx
            .send_replace(balancer_desired_state);

        Ok(())
    }
}

#[async_trait]
impl StateDatabase for File {
    async fn read_balancer_desired_state(
        &self,
    ) -> Result<BalancerDesiredState, StateDatabaseError> {
        self.read_schema_from_file()
            .await
            .map(|schema| schema.balancer_desired_state)
    }

    async fn store_balancer_desired_state(
        &self,
        balancer_desired_state: &BalancerDesiredState,
    ) -> Result<(), StateDatabaseError> {
        let mut schema = self.read_schema_from_file().await?;

        schema.balancer_desired_state = balancer_desired_state.clone();

        self.store_schema(&schema).await
    }
}

#[cfg(test)]
mod tests {
    use std::io::ErrorKind;
    use std::path::PathBuf;

    use tempfile::NamedTempFile;
    use tempfile::TempDir;
    use tokio::fs::metadata;
    use tokio::fs::read_to_string;
    use tokio::fs::write;
    use tokio::sync::watch;

    use paddler_inference_parameters::inference_parameters::InferenceParameters;
    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;
    use paddler_messaging::chat_template::ChatTemplate;

    use super::File;
    use crate::schema::Schema;
    use crate::state_database::StateDatabase;
    use crate::state_database_error::StateDatabaseError;

    #[tokio::test]
    async fn reads_back_the_stored_desired_state_from_the_file() {
        let (balancer_desired_state_notify_tx, _balancer_desired_state_notify_rx) =
            watch::channel(BalancerDesiredState::default());
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("state.json");
        let database = File::new(balancer_desired_state_notify_tx, path.clone());

        let desired_state = BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: BalancerDesiredState::default().inference_parameters,
            model: AgentDesiredModel::LocalToAgent("stored_model_path".to_owned()),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: false,
        };

        database
            .store_balancer_desired_state(&desired_state)
            .await
            .unwrap();

        let read_back = database.read_balancer_desired_state().await.unwrap();

        assert_eq!(read_back.model, desired_state.model);
        assert!(metadata(&path).await.unwrap().is_file());
    }

    #[tokio::test]
    async fn reading_missing_file_stores_and_returns_default_state() {
        let (balancer_desired_state_notify_tx, _balancer_desired_state_notify_rx) =
            watch::channel(BalancerDesiredState::default());
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("not_yet_created.json");
        let database = File::new(balancer_desired_state_notify_tx, path.clone());

        let read_state = database.read_balancer_desired_state().await.unwrap();

        assert_eq!(read_state, BalancerDesiredState::default());
        assert!(metadata(&path).await.unwrap().is_file());
    }

    #[tokio::test]
    async fn reading_invalid_json_returns_parse_error() {
        let (balancer_desired_state_notify_tx, _balancer_desired_state_notify_rx) =
            watch::channel(BalancerDesiredState::default());
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();
        write(&path, b"this is not valid json").await.unwrap();
        let database = File::new(balancer_desired_state_notify_tx, path.clone());

        let read_result = database.read_balancer_desired_state().await;

        assert!(matches!(
            read_result,
            Err(StateDatabaseError::FileContentsInvalid { path: invalid_path, .. }) if invalid_path == path
        ));
    }

    #[tokio::test]
    async fn reading_an_empty_file_fails_and_leaves_the_file_untouched() {
        let (balancer_desired_state_notify_tx, _balancer_desired_state_notify_rx) =
            watch::channel(BalancerDesiredState::default());
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();
        let database = File::new(balancer_desired_state_notify_tx, path.clone());

        let read_result = database.read_balancer_desired_state().await;

        assert!(matches!(
            read_result,
            Err(StateDatabaseError::FileContentsInvalid { path: invalid_path, .. }) if invalid_path == path
        ));
        assert_eq!(read_to_string(&path).await.unwrap(), "");
    }

    #[tokio::test]
    async fn reading_a_directory_path_returns_non_not_found_error() {
        let (balancer_desired_state_notify_tx, _balancer_desired_state_notify_rx) =
            watch::channel(BalancerDesiredState::default());
        let temp_dir = TempDir::new().unwrap();
        let database = File::new(
            balancer_desired_state_notify_tx,
            temp_dir.path().to_path_buf(),
        );

        let read_result = database.read_balancer_desired_state().await;

        assert!(matches!(
            read_result,
            Err(StateDatabaseError::FileReadFailed { source, .. })
                if source.kind() == ErrorKind::IsADirectory
        ));
    }

    #[tokio::test]
    async fn storing_default_state_fails_when_parent_directory_is_missing() {
        let (balancer_desired_state_notify_tx, _balancer_desired_state_notify_rx) =
            watch::channel(BalancerDesiredState::default());
        let temp_dir = TempDir::new().unwrap();
        let path: PathBuf = temp_dir.path().join("missing_directory").join("state.json");
        let database = File::new(balancer_desired_state_notify_tx, path.clone());

        let read_result = database.read_balancer_desired_state().await;

        assert!(matches!(
            read_result,
            Err(StateDatabaseError::FileCreationFailed { path: uncreated_path, source })
                if uncreated_path == path && source.kind() == ErrorKind::NotFound
        ));
    }

    #[tokio::test]
    async fn updating_schema_fails_when_path_is_a_directory() {
        let (balancer_desired_state_notify_tx, _balancer_desired_state_notify_rx) =
            watch::channel(BalancerDesiredState::default());
        let temp_dir = TempDir::new().unwrap();
        let database = File::new(
            balancer_desired_state_notify_tx,
            temp_dir.path().to_path_buf(),
        );

        let store_result = database
            .store_balancer_desired_state(&BalancerDesiredState::default())
            .await;

        assert!(matches!(
            store_result,
            Err(StateDatabaseError::FileReadFailed { source, .. })
                if source.kind() == ErrorKind::IsADirectory
        ));
    }

    #[tokio::test]
    async fn storing_persists_the_state_while_nobody_is_listening() {
        let (balancer_desired_state_notify_tx, balancer_desired_state_notify_rx) =
            watch::channel(BalancerDesiredState::default());
        drop(balancer_desired_state_notify_rx);
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("state.json");
        let database = File::new(balancer_desired_state_notify_tx, path);
        let stored_state = BalancerDesiredState {
            model: AgentDesiredModel::LocalToAgent("stored-model".to_owned()),
            ..BalancerDesiredState::default()
        };

        database
            .store_balancer_desired_state(&stored_state)
            .await
            .unwrap();

        assert_eq!(
            database.read_balancer_desired_state().await.unwrap(),
            stored_state
        );
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn storing_a_small_schema_on_a_character_device_fails_to_sync() {
        let (balancer_desired_state_notify_tx, _balancer_desired_state_notify_rx) =
            watch::channel(BalancerDesiredState::default());
        let database = File::new(balancer_desired_state_notify_tx, PathBuf::from("/dev/full"));

        let store_result = database.store_schema(&Schema::default()).await;

        assert!(matches!(
            store_result,
            Err(StateDatabaseError::FileSyncFailed { source, .. })
                if source.kind() == ErrorKind::InvalidInput
        ));
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn storing_a_large_schema_surfaces_the_write_error_during_write_all() {
        const TOKIO_FILE_BUFFER_BYTES: usize = 2 * 1024 * 1024;

        let (balancer_desired_state_notify_tx, _balancer_desired_state_notify_rx) =
            watch::channel(BalancerDesiredState::default());
        let database = File::new(balancer_desired_state_notify_tx, PathBuf::from("/dev/full"));

        let mut schema = Schema::default();
        schema.balancer_desired_state.model =
            AgentDesiredModel::LocalToAgent("x".repeat(TOKIO_FILE_BUFFER_BYTES * 2));

        let store_result = database.store_schema(&schema).await;

        assert!(matches!(
            store_result,
            Err(StateDatabaseError::FileWriteFailed { source, .. })
                if source.kind() == ErrorKind::StorageFull
        ));
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn storing_to_dev_full_surfaces_permission_denied() {
        let (balancer_desired_state_notify_tx, _balancer_desired_state_notify_rx) =
            watch::channel(BalancerDesiredState::default());
        let database = File::new(balancer_desired_state_notify_tx, PathBuf::from("/dev/full"));

        let store_result = database.store_schema(&Schema::default()).await;

        assert!(matches!(
            store_result,
            Err(StateDatabaseError::FileCreationFailed { source, .. })
                if source.kind() == ErrorKind::PermissionDenied
        ));
    }

    #[tokio::test]
    async fn persists_the_chat_template_override_across_instances() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("state.json");

        let chat_template = ChatTemplate {
            content: "{% for message in messages %}{{ message.content }}{% endfor %}".to_owned(),
        };
        let desired_state = BalancerDesiredState {
            chat_template_override: Some(chat_template.clone()),
            inference_parameters: InferenceParameters::default(),
            model: AgentDesiredModel::LocalToAgent("test_model_path".to_owned()),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: true,
        };

        {
            let (balancer_desired_state_tx, _balancer_desired_state_rx) =
                watch::channel(BalancerDesiredState::default());
            let database = File::new(balancer_desired_state_tx, path.clone());

            database
                .store_balancer_desired_state(&desired_state)
                .await
                .unwrap();
        }

        let (balancer_desired_state_tx, _balancer_desired_state_rx) =
            watch::channel(BalancerDesiredState::default());
        let database = File::new(balancer_desired_state_tx, path);
        let read_back = database.read_balancer_desired_state().await.unwrap();

        assert_eq!(read_back.chat_template_override, Some(chat_template));
        assert!(read_back.use_chat_template_override);
        assert_eq!(read_back.model, desired_state.model);
    }
}
