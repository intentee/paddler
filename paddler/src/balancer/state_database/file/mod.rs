mod schema;

use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use async_trait::async_trait;
use log::warn;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use tokio::fs;
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
    pub fn new(
        balancer_desired_state_notify_tx: broadcast::Sender<BalancerDesiredState>,
        path: PathBuf,
    ) -> Self {
        File {
            balancer_desired_state_notify_tx,
            path,
            write_lock: RwLock::new(()),
        }
    }

    async fn read_schema_from_file(&self) -> Result<Schema> {
        match fs::read_to_string(&self.path).await {
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

        let serialized_schema = serde_json::to_string_pretty(schema)?;
        let mut file = fs::File::create(&self.path).await?;

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
            .balancer_desired_state
            .clone())
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
