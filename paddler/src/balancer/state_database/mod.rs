mod file;
mod memory;

use anyhow::Result;
use async_trait::async_trait;
use paddler_types::balancer_desired_state::BalancerDesiredState;

pub use self::file::File;
pub use self::memory::Memory;

#[async_trait]
pub trait StateDatabase: Send + Sync {
    async fn read_balancer_desired_state(&self) -> Result<BalancerDesiredState>;

    async fn store_balancer_desired_state(&self, state: &BalancerDesiredState) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use paddler_types::agent_desired_model::AgentDesiredModel;
    use paddler_types::inference_parameters::InferenceParameters;
    use tempfile::NamedTempFile;
    use tokio::sync::broadcast;

    use super::*;

    async fn subtest_store_desired_state<TDatabase: StateDatabase>(db: &TDatabase) -> Result<()> {
        let desired_state = BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters::default(),
            model: AgentDesiredModel::LocalToAgent("test_model_path".to_string()),
            use_chat_template_override: false,
        };

        db.store_balancer_desired_state(&desired_state).await?;

        let read_state = db.read_balancer_desired_state().await?;

        assert_eq!(read_state.model, desired_state.model);

        Ok(())
    }

    #[tokio::test]
    async fn test_file_database() -> Result<()> {
        let (balancer_desired_state_tx, _balancer_desired_state_rx) = broadcast::channel(100);
        let tempfile = NamedTempFile::new()?;
        let db = File::new(balancer_desired_state_tx, tempfile.path().to_path_buf());

        subtest_store_desired_state(&db).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_memory_database() -> Result<()> {
        let (balancer_desired_state_tx, _balancer_desired_state_rx) = broadcast::channel(100);
        let db = Memory::new(balancer_desired_state_tx);

        subtest_store_desired_state(&db).await?;

        Ok(())
    }
}
