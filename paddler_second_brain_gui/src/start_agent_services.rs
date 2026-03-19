use std::sync::Arc;

use nanoid::nanoid;
use paddler::agent::continue_from_conversation_history_request::ContinueFromConversationHistoryRequest;
use paddler::agent::continue_from_raw_prompt_request::ContinueFromRawPromptRequest;
use paddler::agent::generate_embedding_batch_request::GenerateEmbeddingBatchRequest;
use paddler::agent::llamacpp_arbiter_service::LlamaCppArbiterService;
use paddler::agent::management_socket_client_service::ManagementSocketClientService;
use paddler::agent::model_metadata_holder::ModelMetadataHolder;
use paddler::agent::reconciliation_service::ReconciliationService;
use paddler::agent_applicable_state_holder::AgentApplicableStateHolder;
use paddler::agent_desired_state::AgentDesiredState;
use paddler::service_manager::ServiceManager;
use paddler::slot_aggregated_status_manager::SlotAggregatedStatusManager;
use paddler_types::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

use crate::agent_status_monitor_service::AgentStatusMonitorService;

pub async fn start_agent_services(
    agent_name: Option<String>,
    management_address: String,
    slots: i32,
    agent_status_tx: mpsc::UnboundedSender<SlotAggregatedStatusSnapshot>,
    shutdown_rx: oneshot::Receiver<()>,
) -> anyhow::Result<()> {
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
    let mut service_manager = ServiceManager::default();
    let slot_aggregated_status_manager = Arc::new(SlotAggregatedStatusManager::new(slots));

    service_manager.add_service(AgentStatusMonitorService {
        agent_status_tx,
        slot_aggregated_status: slot_aggregated_status_manager
            .slot_aggregated_status
            .clone(),
    });

    service_manager.add_service(LlamaCppArbiterService {
        agent_applicable_state: None,
        agent_applicable_state_holder: agent_applicable_state_holder.clone(),
        agent_name: agent_name.clone(),
        continue_from_conversation_history_request_rx,
        continue_from_raw_prompt_request_rx,
        desired_slots_total: slots,
        generate_embedding_batch_request_rx,
        llamacpp_arbiter_handle: None,
        model_metadata_holder: model_metadata_holder.clone(),
        slot_aggregated_status_manager: slot_aggregated_status_manager.clone(),
    });

    service_manager.add_service(ManagementSocketClientService {
        agent_applicable_state_holder: agent_applicable_state_holder.clone(),
        agent_desired_state_tx,
        continue_from_conversation_history_request_tx,
        continue_from_raw_prompt_request_tx,
        generate_embedding_batch_request_tx,
        model_metadata_holder,
        name: agent_name,
        receive_stream_stopper_collection: Default::default(),
        slot_aggregated_status: slot_aggregated_status_manager
            .slot_aggregated_status
            .clone(),
        socket_url: format!(
            "ws://{}/api/v1/agent_socket/{}",
            management_address,
            nanoid!()
        ),
    });

    service_manager.add_service(ReconciliationService {
        agent_applicable_state_holder,
        agent_desired_state: None,
        agent_desired_state_rx,
        is_converted_to_applicable_state: false,
        slot_aggregated_status: slot_aggregated_status_manager
            .slot_aggregated_status
            .clone(),
    });

    service_manager.run_forever(shutdown_rx).await
}
