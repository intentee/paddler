mod file;
mod memory;

use anyhow::Result;
use async_trait::async_trait;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;

pub use self::file::File;
pub use self::memory::Memory;

#[async_trait]
pub trait StateDatabase: Send + Sync {
    async fn read_balancer_desired_state(&self) -> Result<BalancerDesiredState>;

    async fn store_balancer_desired_state(&self, state: &BalancerDesiredState) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::chat_template::ChatTemplate;
    use paddler_messaging::inference_parameters::InferenceParameters;
    use tempfile::NamedTempFile;
    use tokio::sync::broadcast;

    use super::*;

    async fn subtest_store_desired_state<TDatabase: StateDatabase>(database: &TDatabase) {
        let desired_state = BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters::default(),
            model: AgentDesiredModel::LocalToAgent("test_model_path".to_owned()),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: false,
        };

        database
            .store_balancer_desired_state(&desired_state)
            .await
            .unwrap();

        let read_state = database.read_balancer_desired_state().await.unwrap();

        assert_eq!(read_state.model, desired_state.model);
    }

    #[tokio::test]
    async fn test_file_database() {
        let (balancer_desired_state_tx, _balancer_desired_state_rx) = broadcast::channel(100);
        let tempfile = NamedTempFile::new().unwrap();
        let database = File::new(balancer_desired_state_tx, tempfile.path().to_path_buf());

        subtest_store_desired_state(&database).await;
    }

    #[tokio::test]
    async fn test_memory_database() {
        let (balancer_desired_state_tx, _balancer_desired_state_rx) = broadcast::channel(100);
        let database = Memory::new(balancer_desired_state_tx, BalancerDesiredState::default());

        subtest_store_desired_state(&database).await;
    }

    #[tokio::test]
    async fn test_file_database_persists_chat_template_override_across_fresh_instance() {
        let tempfile = NamedTempFile::new().unwrap();
        let path = tempfile.path().to_path_buf();

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
            let (balancer_desired_state_tx, _balancer_desired_state_rx) = broadcast::channel(100);
            let database = File::new(balancer_desired_state_tx, path.clone());

            database
                .store_balancer_desired_state(&desired_state)
                .await
                .unwrap();
        }

        let (balancer_desired_state_tx, _balancer_desired_state_rx) = broadcast::channel(100);
        let database = File::new(balancer_desired_state_tx, path);
        let read_back = database.read_balancer_desired_state().await.unwrap();

        assert_eq!(read_back.chat_template_override, Some(chat_template));
        assert!(read_back.use_chat_template_override);
        assert_eq!(read_back.model, desired_state.model);
    }
}
