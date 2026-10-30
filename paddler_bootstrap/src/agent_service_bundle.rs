use std::sync::Arc;

use nanoid::nanoid;
use tokio::sync::watch;
use trzcina::ServiceManager;

use paddler_agent::agent_applicable_state_holder::AgentApplicableStateHolder;
use paddler_agent::balancer_message_context::BalancerMessageContext;
use paddler_agent::desired_state_reconciler::DesiredStateReconciler;
use paddler_agent::llamacpp_arbiter_service::LlamaCppArbiterService;
use paddler_agent::management_socket_client_service::ManagementSocketClientService;
use paddler_agent::reconciliation_service::ReconciliationService;
use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_runtime::inference_runtime_context_params::InferenceRuntimeContextParams;
use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::api_path::ApiPath;
use paddler_outbox::outbox::Outbox;

use crate::agent_bootstrap_config::AgentBootstrapConfig;
use crate::registers_services::RegistersServices;

pub struct AgentServiceBundle {
    llamacpp_arbiter: LlamaCppArbiterService,
    management_socket_client: ManagementSocketClientService,
    reconciliation: ReconciliationService,
}

impl AgentServiceBundle {
    #[must_use]
    pub fn new(
        AgentBootstrapConfig {
            agent_name,
            desired_model_resolver,
            management_address,
            slots,
        }: AgentBootstrapConfig,
    ) -> Self {
        let (agent_desired_state_tx, mut agent_desired_state_rx) =
            watch::channel(AgentDesiredState::default());

        agent_desired_state_rx.mark_changed();

        let pipeline_requests = Arc::new(Outbox::default());

        let agent_applicable_state_holder = Arc::new(AgentApplicableStateHolder::default());
        let served_applicable_state_holder = Arc::new(AgentApplicableStateHolder::default());
        let inference_runtime_context =
            InferenceRuntimeContext::new(InferenceRuntimeContextParams {
                agent_name: agent_name.clone(),
                slots,
            });
        let model_metadata_holder = inference_runtime_context.model_metadata_holder.clone();
        let slot_aggregated_status = inference_runtime_context.slot_aggregated_status.clone();

        let llamacpp_arbiter = LlamaCppArbiterService {
            agent_applicable_state_holder: agent_applicable_state_holder.clone(),
            inference_runtime_context,
            pipeline_requests: pipeline_requests.clone(),
            served_applicable_state_holder: served_applicable_state_holder.clone(),
        };

        let management_socket_client = ManagementSocketClientService {
            balancer_message_context: BalancerMessageContext {
                agent_desired_state_tx,
                model_metadata_holder,
                pipeline_requests,
                request_stoppers: Arc::default(),
                served_applicable_state_holder,
                slot_aggregated_status: slot_aggregated_status.clone(),
            },
            name: agent_name,
            socket_url: format!(
                "ws://{management_address}{}",
                ApiPath::agent_socket(&nanoid!())
            ),
        };

        let reconciliation = ReconciliationService {
            agent_applicable_state_holder,
            agent_desired_state_rx,
            desired_state_reconciler: DesiredStateReconciler {
                desired_model_resolver,
                slot_aggregated_status,
            },
        };

        Self {
            llamacpp_arbiter,
            management_socket_client,
            reconciliation,
        }
    }
}

impl RegistersServices for AgentServiceBundle {
    fn register_services(self, service_manager: &mut ServiceManager) {
        service_manager.register_service(self.llamacpp_arbiter);
        service_manager.register_service(self.management_socket_client);
        service_manager.register_service(self.reconciliation);
    }
}
