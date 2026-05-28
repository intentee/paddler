use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicI32;
use std::sync::atomic::AtomicU64;

use paddler::atomic_value::AtomicValue;
use paddler::balancer::agent_controller::AgentController;
use paddler::balancer::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
use paddler::balancer::embedding_sender_collection::EmbeddingSenderCollection;
use paddler::balancer::generate_tokens_sender_collection::GenerateTokensSenderCollection;
use paddler::balancer::model_metadata_sender_collection::ModelMetadataSenderCollection;
use paddler::agent_state_application_status::AgentStateApplicationStatus;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[must_use]
pub fn make_agent_controller_without_remote_agent(id: &str) -> AgentController {
    let (agent_message_tx, _agent_message_rx) = mpsc::unbounded_channel();

    AgentController {
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
        generate_tokens_sender_collection: Arc::new(GenerateTokensSenderCollection::default()),
        id: id.to_owned(),
        issues: RwLock::new(BTreeSet::new()),
        model_metadata_sender_collection: Arc::new(ModelMetadataSenderCollection::default()),
        model_path: RwLock::new(None),
        name: None,
        newest_update_version: AtomicValue::<AtomicI32>::new(0),
        slots_processing: AtomicValue::<AtomicI32>::new(0),
        slots_total: AtomicValue::<AtomicI32>::new(0),
        state_application_status_code: AtomicValue::<AtomicI32>::new(
            AgentStateApplicationStatus::Fresh as i32,
        ),
        uses_chat_template_override: AtomicValue::<AtomicBool>::new(false),
    }
}
