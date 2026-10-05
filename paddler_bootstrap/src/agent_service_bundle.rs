use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use nanoid::nanoid;
use tokio::sync::mpsc;
use tokio::sync::watch;
use trzcina::Service;
use trzcina::ServiceBundle;

use paddler_agent::agent_applicable_state_holder::AgentApplicableStateHolder;
use paddler_agent::balancer_message_context::BalancerMessageContext;
use paddler_agent::continuous_batch_arbiter_context::ContinuousBatchArbiterContext;
use paddler_agent::continuous_batch_preparation_request::ContinuousBatchPreparationRequest;
use paddler_agent::desired_state_reconciler::DesiredStateReconciler;
use paddler_agent::llamacpp_arbiter_service::LlamaCppArbiterService;
use paddler_agent::management_socket_client_service::ManagementSocketClientService;
use paddler_agent::model_metadata_holder::ModelMetadataHolder;
use paddler_agent::reconciliation_service::ReconciliationService;
use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::api_path::ApiPath;
use paddler_messaging::balancer_connection::BalancerConnection;

use crate::agent_bootstrap_config::AgentBootstrapConfig;

pub struct AgentServiceBundle {
    pub balancer_connection_rx: watch::Receiver<BalancerConnection>,
    pub slot_aggregated_status: Arc<SlotAggregatedStatus>,
    llamacpp_arbiter_service: LlamaCppArbiterService,
    management_socket_client_service: ManagementSocketClientService,
    reconciliation_service: ReconciliationService,
}

impl AgentServiceBundle {
    #[must_use]
    pub fn new(
        AgentBootstrapConfig {
            agent_name,
            management_address,
            slots,
        }: AgentBootstrapConfig,
    ) -> Self {
        let (balancer_connection_tx, balancer_connection_rx) =
            watch::channel(BalancerConnection::Connecting);
        let (agent_desired_state_tx, agent_desired_state_rx) =
            mpsc::unbounded_channel::<AgentDesiredState>();
        let (continuous_batch_preparation_request_tx, continuous_batch_preparation_request_rx) =
            mpsc::unbounded_channel::<ContinuousBatchPreparationRequest>();

        let agent_applicable_state_holder = Arc::new(AgentApplicableStateHolder::default());
        let model_metadata_holder = Arc::new(ModelMetadataHolder::default());
        let slot_aggregated_status = Arc::new(SlotAggregatedStatus::new(slots));

        let llamacpp_arbiter_service = LlamaCppArbiterService {
            agent_applicable_state_holder: agent_applicable_state_holder.clone(),
            arbiter_context: ContinuousBatchArbiterContext {
                agent_name: agent_name.clone(),
                model_metadata_holder: model_metadata_holder.clone(),
                slot_aggregated_status: slot_aggregated_status.clone(),
            },
            continuous_batch_preparation_request_rx,
        };

        let management_socket_client_service = ManagementSocketClientService {
            balancer_connection_tx,
            balancer_message_context: BalancerMessageContext {
                agent_applicable_state_holder: agent_applicable_state_holder.clone(),
                agent_desired_state_tx,
                continuous_batch_preparation_request_tx,
                model_metadata_holder,
                request_stoppers: Arc::default(),
                slot_aggregated_status: slot_aggregated_status.clone(),
            },
            name: agent_name,
            socket_url: format!(
                "ws://{management_address}{}",
                ApiPath::agent_socket(&nanoid!())
            ),
        };

        let reconciliation_service = ReconciliationService {
            agent_desired_state_rx,
            desired_state_reconciler: DesiredStateReconciler {
                agent_applicable_state_holder,
                slot_aggregated_status: slot_aggregated_status.clone(),
            },
        };

        Self {
            balancer_connection_rx,
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
