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
use paddler_agent::desired_state_reconciler::DesiredStateReconciler;
use paddler_agent::llamacpp_arbiter_service::LlamaCppArbiterService;
use paddler_agent::management_socket_client_service::ManagementSocketClientService;
use paddler_agent::pipeline_request::PipelineRequest;
use paddler_agent::reconciliation_service::ReconciliationService;
use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_runtime::model_metadata_holder::ModelMetadataHolder;
use paddler_agent_runtime::shared_llama_backend::SharedLlamaBackend;
use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::api_path::ApiPath;

use crate::agent_bootstrap_config::AgentBootstrapConfig;
use paddler_agent_runtime::system_parallelism::SystemParallelism;

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
            cache_dir,
            management_address,
            slots,
        }: AgentBootstrapConfig,
    ) -> Self {
        let (agent_desired_state_tx, mut agent_desired_state_rx) =
            watch::channel(AgentDesiredState::default());

        agent_desired_state_rx.mark_changed();

        let (pipeline_request_tx, pipeline_request_rx) =
            mpsc::unbounded_channel::<PipelineRequest>();

        let agent_applicable_state_holder = Arc::new(AgentApplicableStateHolder::default());
        let model_metadata_holder = Arc::new(ModelMetadataHolder::default());
        let served_applicable_state_holder = Arc::new(AgentApplicableStateHolder::default());
        let slot_aggregated_status = Arc::new(SlotAggregatedStatus::new(slots));

        let llamacpp_arbiter = LlamaCppArbiterService {
            agent_applicable_state_holder: agent_applicable_state_holder.clone(),
            inference_runtime_context: InferenceRuntimeContext {
                agent_name: agent_name.clone(),
                available_parallelism: Arc::new(SystemParallelism),
                llama_backend: Arc::new(SharedLlamaBackend::default()),
                model_metadata_holder: model_metadata_holder.clone(),
                slot_aggregated_status: slot_aggregated_status.clone(),
            },
            pipeline_request_rx,
            served_applicable_state_holder: served_applicable_state_holder.clone(),
        };

        let management_socket_client = ManagementSocketClientService {
            balancer_message_context: BalancerMessageContext {
                agent_desired_state_tx,
                model_metadata_holder,
                pipeline_request_tx,
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
            agent_desired_state_rx,
            desired_state_reconciler: DesiredStateReconciler {
                agent_applicable_state_holder,
                cache_dir,
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

#[async_trait]
impl ServiceBundle for AgentServiceBundle {
    async fn services(self) -> Result<Vec<Box<dyn Service>>> {
        Ok(vec![
            Box::new(self.llamacpp_arbiter),
            Box::new(self.management_socket_client),
            Box::new(self.reconciliation),
        ])
    }
}
