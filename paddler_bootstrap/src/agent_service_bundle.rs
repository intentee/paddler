use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use nanoid::nanoid;
use paddler::agent::continue_from_conversation_history_request::ContinueFromConversationHistoryRequest;
use paddler::agent::continue_from_raw_prompt_request::ContinueFromRawPromptRequest;
use paddler::agent::generate_embedding_batch_request::GenerateEmbeddingBatchRequest;
use paddler::agent::llamacpp_arbiter_service::LlamaCppArbiterService;
use paddler::agent::management_socket_client_service::ManagementSocketClientService;
use paddler::agent::model_metadata_holder::ModelMetadataHolder;
use paddler::agent::reconciliation_service::ReconciliationService;
use paddler::agent_applicable_state_holder::AgentApplicableStateHolder;
use paddler::slot_aggregated_status::SlotAggregatedStatus;
use paddler::slot_aggregated_status_manager::SlotAggregatedStatusManager;
use paddler_types::agent_desired_state::AgentDesiredState;
use tokio::sync::mpsc;
use trzcina::Service;
use trzcina::ServiceBundle;

pub struct AgentServiceBundle {
    pub slot_aggregated_status: Arc<SlotAggregatedStatus>,
    llamacpp_arbiter_service: LlamaCppArbiterService,
    management_socket_client_service: ManagementSocketClientService,
    reconciliation_service: ReconciliationService,
}

impl AgentServiceBundle {
    #[must_use]
    pub fn new(agent_name: Option<String>, management_address: &str, slots: i32) -> Self {
        let (agent_desired_state_tx, agent_desired_state_rx) =
            mpsc::unbounded_channel::<AgentDesiredState>();
        let (
            continue_from_conversation_history_request_tx,
            continue_from_conversation_history_request_rx,
        ) = mpsc::unbounded_channel::<ContinueFromConversationHistoryRequest>();
        let (continue_from_raw_prompt_request_tx, continue_from_raw_prompt_request_rx) =
            mpsc::unbounded_channel::<ContinueFromRawPromptRequest>();
        let (generate_embedding_batch_request_tx, generate_embedding_batch_request_rx) =
            mpsc::unbounded_channel::<GenerateEmbeddingBatchRequest>();

        let agent_applicable_state_holder = Arc::new(AgentApplicableStateHolder::default());
        let model_metadata_holder = Arc::new(ModelMetadataHolder::default());
        let slot_aggregated_status_manager = Arc::new(SlotAggregatedStatusManager::new(slots));
        let slot_aggregated_status = slot_aggregated_status_manager.slot_aggregated_status.clone();

        let llamacpp_arbiter_service = LlamaCppArbiterService {
            agent_applicable_state: None,
            agent_applicable_state_holder: agent_applicable_state_holder.clone(),
            agent_name: agent_name.clone(),
            continue_from_conversation_history_request_rx,
            continue_from_raw_prompt_request_rx,
            desired_slots_total: slots,
            generate_embedding_batch_request_rx,
            continuous_batch_arbiter_handle: None,
            model_metadata_holder: model_metadata_holder.clone(),
            slot_aggregated_status_manager,
        };

        let management_socket_client_service = ManagementSocketClientService {
            agent_applicable_state_holder: agent_applicable_state_holder.clone(),
            agent_desired_state_tx,
            continue_from_conversation_history_request_tx,
            continue_from_raw_prompt_request_tx,
            generate_embedding_batch_request_tx,
            model_metadata_holder,
            name: agent_name,
            receive_stream_stopper_collection: Arc::default(),
            slot_aggregated_status: slot_aggregated_status.clone(),
            socket_url: format!(
                "ws://{}/api/v1/agent_socket/{}",
                management_address,
                nanoid!()
            ),
        };

        let reconciliation_service = ReconciliationService {
            agent_applicable_state_holder,
            agent_desired_state: None,
            agent_desired_state_rx,
            is_converted_to_applicable_state: false,
            slot_aggregated_status: slot_aggregated_status.clone(),
        };

        Self {
            slot_aggregated_status,
            llamacpp_arbiter_service,
            management_socket_client_service,
            reconciliation_service,
        }
    }
}

#[async_trait]
impl ServiceBundle for AgentServiceBundle {
    async fn services(self) -> Result<Vec<Box<dyn Service>>> {
        Ok(vec![
            Box::new(self.llamacpp_arbiter_service),
            Box::new(self.management_socket_client_service),
            Box::new(self.reconciliation_service),
        ])
    }
}
