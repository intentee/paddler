use std::sync::Arc;

use log::error;
use log::info;

use crate::agent_controller_pool::AgentControllerPool;
use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use crate::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
use crate::embedding_sender_collection::EmbeddingSenderCollection;
use crate::generate_tokens_sender_collection::GenerateTokensSenderCollection;
use crate::model_metadata_sender_collection::ModelMetadataSenderCollection;

pub struct AgentSocketControllerContext {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub agent_id: String,
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    pub chat_template_override_sender_collection: Arc<ChatTemplateOverrideSenderCollection>,
    pub embedding_sender_collection: Arc<EmbeddingSenderCollection>,
    pub generate_tokens_sender_collection: Arc<GenerateTokensSenderCollection>,
    pub model_metadata_sender_collection: Arc<ModelMetadataSenderCollection>,
}

impl Drop for AgentSocketControllerContext {
    fn drop(&mut self) {
        if let Err(err) = self
            .agent_controller_pool
            .remove_agent_controller(&self.agent_id)
        {
            error!("Failed to remove agent: {err}");
        }

        info!("Removed agent: {}", self.agent_id);
    }
}

#[cfg(test)]
mod tests {
    use parking_lot::RwLock;
    use std::collections::BTreeSet;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::AtomicI32;
    use std::sync::atomic::AtomicU64;

    use tokio::sync::mpsc;
    use tokio_util::sync::CancellationToken;

    use super::AgentSocketControllerContext;
    use crate::agent_controller::AgentController;
    use crate::agent_controller_pool::AgentControllerPool;
    use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
    use crate::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
    use crate::embedding_sender_collection::EmbeddingSenderCollection;
    use crate::generate_tokens_sender_collection::GenerateTokensSenderCollection;
    use crate::model_metadata_sender_collection::ModelMetadataSenderCollection;
    use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
    use paddler_messaging::atomic_value::AtomicValue;

    #[test]
    fn drop_removes_registered_agent_from_pool() {
        let agent_controller_pool = Arc::new(AgentControllerPool::default());
        let (agent_message_tx, _agent_message_rx) = mpsc::unbounded_channel();

        agent_controller_pool
            .register_agent_controller(
                "agent-under-drop".to_owned(),
                Arc::new(AgentController {
                    agent_message_tx,
                    chat_template_override_sender_collection: Arc::new(
                        ChatTemplateOverrideSenderCollection::default(),
                    ),
                    connection_close: CancellationToken::new(),
                    desired_slots_total: AtomicValue::<AtomicI32>::new(0),
                    download_current: AtomicValue::<AtomicU64>::new(0),
                    download_filename: RwLock::new(None),
                    download_indeterminate: AtomicValue::<AtomicBool>::new(true),
                    download_total: AtomicValue::<AtomicU64>::new(0),
                    embedding_sender_collection: Arc::new(EmbeddingSenderCollection::default()),
                    generate_tokens_sender_collection: Arc::new(
                        GenerateTokensSenderCollection::default(),
                    ),
                    id: "agent-under-drop".to_owned(),
                    issues: RwLock::new(BTreeSet::new()),
                    model_metadata_sender_collection: Arc::new(
                        ModelMetadataSenderCollection::default(),
                    ),
                    model_path: RwLock::new(None),
                    name: None,
                    newest_update_version: AtomicValue::<AtomicI32>::new(0),
                    slots_processing: AtomicValue::<AtomicI32>::new(0),
                    slots_total: AtomicValue::<AtomicI32>::new(1),
                    state_application_status_code: AtomicValue::<AtomicI32>::new(
                        AgentStateApplicationStatus::Fresh as i32,
                    ),
                    uses_chat_template_override: AtomicValue::<AtomicBool>::new(false),
                }),
            )
            .unwrap();

        let context = AgentSocketControllerContext {
            agent_controller_pool: agent_controller_pool.clone(),
            agent_id: "agent-under-drop".to_owned(),
            balancer_applicable_state_holder: Arc::new(BalancerApplicableStateHolder::default()),
            chat_template_override_sender_collection: Arc::new(
                ChatTemplateOverrideSenderCollection::default(),
            ),
            embedding_sender_collection: Arc::new(EmbeddingSenderCollection::default()),
            generate_tokens_sender_collection: Arc::new(GenerateTokensSenderCollection::default()),
            model_metadata_sender_collection: Arc::new(ModelMetadataSenderCollection::default()),
        };

        assert!(
            agent_controller_pool
                .get_agent_controller("agent-under-drop")
                .is_some()
        );

        drop(context);

        assert!(
            agent_controller_pool
                .get_agent_controller("agent-under-drop")
                .is_none()
        );
    }
}
