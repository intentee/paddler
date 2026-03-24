use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use paddler::balancer::agent_controller_pool::AgentControllerPool;
use paddler::balancer::buffered_request_manager::BufferedRequestManager;
use paddler::balancer::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
use paddler::balancer::embedding_sender_collection::EmbeddingSenderCollection;
use paddler::balancer::generate_tokens_sender_collection::GenerateTokensSenderCollection;
use paddler::balancer::inference_service::InferenceService;
use paddler::balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use paddler::balancer::management_service::ManagementService;
use paddler::balancer::management_service::configuration::Configuration as ManagementServiceConfiguration;
use paddler::balancer::model_metadata_sender_collection::ModelMetadataSenderCollection;
use paddler::balancer::reconciliation_service::ReconciliationService;
use paddler::balancer::state_database::Memory;
use paddler::balancer::state_database::StateDatabase;
use paddler::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use paddler::service_manager::ServiceManager;
use paddler_types::agent_controller_snapshot::AgentControllerSnapshot;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

use crate::agent_monitor_service::AgentMonitorService;

pub async fn start_balancer_services(
    management_addr: SocketAddr,
    inference_addr: SocketAddr,
    initial_desired_state: BalancerDesiredState,
    agent_snapshots_tx: mpsc::UnboundedSender<Vec<AgentControllerSnapshot>>,
    shutdown_rx: oneshot::Receiver<()>,
) -> anyhow::Result<()> {
    let (balancer_desired_state_tx, balancer_desired_state_rx) = broadcast::channel(100);

    let agent_controller_pool = Arc::new(AgentControllerPool::default());
    let balancer_applicable_state_holder = Arc::new(BalancerApplicableStateHolder::default());
    let buffered_request_manager = Arc::new(BufferedRequestManager::new(
        agent_controller_pool.clone(),
        Duration::from_millis(10000),
        30,
    ));
    let chat_template_override_sender_collection =
        Arc::new(ChatTemplateOverrideSenderCollection::default());
    let embedding_sender_collection = Arc::new(EmbeddingSenderCollection::default());
    let generate_tokens_sender_collection = Arc::new(GenerateTokensSenderCollection::default());
    let model_metadata_sender_collection = Arc::new(ModelMetadataSenderCollection::default());
    let mut service_manager = ServiceManager::default();
    let state_database: Arc<dyn StateDatabase> =
        Arc::new(Memory::new(balancer_desired_state_tx.clone()));

    state_database
        .store_balancer_desired_state(&initial_desired_state)
        .await?;

    service_manager.add_service(InferenceService {
        balancer_applicable_state_holder: balancer_applicable_state_holder.clone(),
        buffered_request_manager: buffered_request_manager.clone(),
        configuration: InferenceServiceConfiguration {
            addr: inference_addr,
            cors_allowed_hosts: vec![],
            inference_item_timeout: Duration::from_millis(30000),
        },
        #[cfg(feature = "web_admin_panel")]
        web_admin_panel_service_configuration: None,
    });

    service_manager.add_service(ManagementService {
        agent_controller_pool: agent_controller_pool.clone(),
        balancer_applicable_state_holder: balancer_applicable_state_holder.clone(),
        buffered_request_manager: buffered_request_manager.clone(),
        chat_template_override_sender_collection,
        configuration: ManagementServiceConfiguration {
            addr: management_addr,
            cors_allowed_hosts: vec![],
        },
        embedding_sender_collection,
        generate_tokens_sender_collection,
        model_metadata_sender_collection,
        state_database: state_database.clone(),
        statsd_prefix: "paddler_".to_string(),
        #[cfg(feature = "web_admin_panel")]
        web_admin_panel_service_configuration: None,
    });

    service_manager.add_service(AgentMonitorService {
        agent_controller_pool: agent_controller_pool.clone(),
        agent_snapshots_tx,
    });

    service_manager.add_service(ReconciliationService {
        agent_controller_pool: agent_controller_pool.clone(),
        balancer_applicable_state_holder,
        balancer_desired_state: state_database.read_balancer_desired_state().await?,
        balancer_desired_state_rx,
        is_converted_to_applicable_state: false,
    });

    service_manager.run_forever(shutdown_rx).await
}
