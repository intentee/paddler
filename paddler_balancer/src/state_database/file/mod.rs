mod schema;

use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use async_trait::async_trait;
use log::warn;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use tokio::fs::read_to_string;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
use tokio::sync::broadcast;

use self::schema::Schema;
use super::StateDatabase;

pub struct File {
    balancer_desired_state_notify_tx: broadcast::Sender<BalancerDesiredState>,
    path: PathBuf,
    write_lock: RwLock<()>,
}

impl File {
    #[must_use]
    pub fn new(
        balancer_desired_state_notify_tx: broadcast::Sender<BalancerDesiredState>,
        path: PathBuf,
    ) -> Self {
        Self {
            balancer_desired_state_notify_tx,
            path,
            write_lock: RwLock::new(()),
        }
    }

    async fn read_schema_from_file(&self) -> Result<Schema> {
        match read_to_string(&self.path).await {
            Ok(content) => {
                if content.is_empty() {
                    return self.store_default_schema().await;
                }

                let schema: Schema = serde_json::from_str(&content).context(format!("Unable to parse database file contents: '{}'. Either that is not a valid database file, or this version of Paddler is incompatible with it.", self.path.display()))?;

                Ok(schema)
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                warn!(
                    "State database file not found; trying to store the default state: '{}'",
                    self.path.display()
                );

                self.store_default_schema().await
            }
            Err(err) => Err(err.into()),
        }
    }

    async fn store_default_schema(&self) -> Result<Schema> {
        let schema = Schema::default();

        self.store_schema(&schema)
            .await
            .context("Failed to store default state")?;

        Ok(schema)
    }

    async fn store_schema(&self, schema: &Schema) -> Result<()> {
        let balancer_desired_state = schema.balancer_desired_state.clone();
        let _lock = self.write_lock.write().await;

        let serialized_schema = serde_json::to_string_pretty(schema)
            .context("Failed to serialize the state database schema")?;
        let mut file = tokio::fs::File::create(&self.path).await?;

        file.write_all(serialized_schema.as_bytes()).await?;
        file.sync_all().await?;

        self.balancer_desired_state_notify_tx
            .send(balancer_desired_state)?;

        Ok(())
    }

    async fn update_schema<TModifier>(&self, modifier: TModifier) -> Result<()>
    where
        TModifier: FnOnce(&mut Schema),
    {
        let mut schema = self
            .read_schema_from_file()
            .await
            .context("Unable to read current state from file")?;

        modifier(&mut schema);

        self.store_schema(&schema).await
    }
}

#[async_trait]
impl StateDatabase for File {
    async fn read_balancer_desired_state(&self) -> Result<BalancerDesiredState> {
        Ok(self
            .read_schema_from_file()
            .await
            .context("Unable to read state from file")?
            .balancer_desired_state)
    }

    async fn store_balancer_desired_state(
        &self,
        balancer_desired_state: &BalancerDesiredState,
    ) -> Result<()> {
        self.update_schema(|schema| {
            schema.balancer_desired_state = balancer_desired_state.clone();
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use log::LevelFilter;
    use tempfile::NamedTempFile;
    use tempfile::TempDir;
    use tokio::fs::metadata;
    use tokio::fs::write;
    use tokio::sync::broadcast;

    use super::File;
    use super::schema::Schema;
    use crate::state_database::StateDatabase;
    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;

    #[tokio::test]
    async fn store_then_read_round_trips_through_real_file() {
        let (balancer_desired_state_notify_tx, _balancer_desired_state_notify_rx) =
            broadcast::channel(8);
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
            broadcast::channel(8);
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
            broadcast::channel(8);
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();
        write(&path, b"this is not valid json").await.unwrap();
        let database = File::new(balancer_desired_state_notify_tx, path);

        let read_result = database.read_balancer_desired_state().await;

        assert!(read_result.is_err());
    }

    #[tokio::test]
    async fn reading_a_directory_path_returns_non_not_found_error() {
        let (balancer_desired_state_notify_tx, _balancer_desired_state_notify_rx) =
            broadcast::channel(8);
        let temp_dir = TempDir::new().unwrap();
        let database = File::new(
            balancer_desired_state_notify_tx,
            temp_dir.path().to_path_buf(),
        );

        let read_result = database.read_balancer_desired_state().await;

        assert!(read_result.is_err());
    }

    #[tokio::test]
    async fn storing_default_state_fails_when_parent_directory_is_missing() {
        let (balancer_desired_state_notify_tx, _balancer_desired_state_notify_rx) =
            broadcast::channel(8);
        let temp_dir = TempDir::new().unwrap();
        let path: PathBuf = temp_dir.path().join("missing_directory").join("state.json");
        let database = File::new(balancer_desired_state_notify_tx, path);

        let read_result = database.read_balancer_desired_state().await;

        assert!(read_result.is_err());
    }

    #[tokio::test]
    async fn updating_schema_fails_when_path_is_a_directory() {
        let (balancer_desired_state_notify_tx, _balancer_desired_state_notify_rx) =
            broadcast::channel(8);
        let temp_dir = TempDir::new().unwrap();
        let database = File::new(
            balancer_desired_state_notify_tx,
            temp_dir.path().to_path_buf(),
        );

        let store_result = database
            .store_balancer_desired_state(&BalancerDesiredState::default())
            .await;

        assert!(store_result.is_err());
    }

    #[tokio::test]
    async fn storing_fails_when_no_receivers_are_listening() {
        let (balancer_desired_state_notify_tx, balancer_desired_state_notify_rx) =
            broadcast::channel(8);
        drop(balancer_desired_state_notify_rx);
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("state.json");
        let database = File::new(balancer_desired_state_notify_tx, path);

        let store_result = database
            .store_balancer_desired_state(&BalancerDesiredState::default())
            .await;

        assert!(store_result.is_err());
    }

    #[tokio::test]
    async fn reading_missing_file_logs_path_when_warnings_are_enabled() {
        log::set_max_level(LevelFilter::Warn);

        let (balancer_desired_state_notify_tx, _balancer_desired_state_notify_rx) =
            broadcast::channel(8);
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("warned_missing.json");
        let database = File::new(balancer_desired_state_notify_tx, path.clone());

        let read_state = database.read_balancer_desired_state().await.unwrap();

        assert_eq!(read_state, BalancerDesiredState::default());
        assert!(metadata(&path).await.unwrap().is_file());
    }

    #[tokio::test]
    async fn storing_schema_fails_when_target_is_unwritable() {
        let (balancer_desired_state_notify_tx, _balancer_desired_state_notify_rx) =
            broadcast::channel(8);
        let database = File::new(balancer_desired_state_notify_tx, PathBuf::from("/dev/full"));

        let store_result = database.store_schema(&Schema::default()).await;

        let store_error = store_result.err().unwrap();

        assert!(store_error.downcast_ref::<std::io::Error>().is_some());
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn storing_a_large_schema_surfaces_the_write_error_during_write_all() {
        const TOKIO_FILE_BUFFER_BYTES: usize = 2 * 1024 * 1024;

        let (balancer_desired_state_notify_tx, _balancer_desired_state_notify_rx) =
            broadcast::channel(8);
        let database = File::new(balancer_desired_state_notify_tx, PathBuf::from("/dev/full"));

        let mut schema = Schema::default();
        schema.balancer_desired_state.model =
            AgentDesiredModel::LocalToAgent("x".repeat(TOKIO_FILE_BUFFER_BYTES * 2));

        let store_result = database.store_schema(&schema).await;

        let store_error = store_result.err().unwrap();
        let io_error = store_error.downcast_ref::<std::io::Error>().unwrap();

        assert_eq!(io_error.kind(), std::io::ErrorKind::StorageFull);
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn storing_to_dev_full_surfaces_permission_denied() {
        let (balancer_desired_state_notify_tx, _balancer_desired_state_notify_rx) =
            broadcast::channel(8);
        let database = File::new(balancer_desired_state_notify_tx, PathBuf::from("/dev/full"));

        let store_result = database.store_schema(&Schema::default()).await;

        let store_error = store_result.err().unwrap();
        let io_error = store_error.downcast_ref::<std::io::Error>().unwrap();

        assert_eq!(io_error.kind(), std::io::ErrorKind::PermissionDenied);
    }
}
