use std::sync::Arc;
use std::time::Duration;

use paddler::balancer::agent_controller_pool::AgentControllerPool;
use paddler::balancer::buffered_request_manager::BufferedRequestManager;
use paddler::balancer::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
use paddler::balancer::compatibility::openai_service::OpenAIService;
use paddler::balancer::compatibility::openai_service::configuration::Configuration as OpenAIServiceConfiguration;
use paddler::balancer::embedding_sender_collection::EmbeddingSenderCollection;
use paddler::balancer::generate_tokens_sender_collection::GenerateTokensSenderCollection;
use paddler::balancer::inference_service::InferenceService;
use paddler::balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use paddler::balancer::management_service::ManagementService;
use paddler::balancer::management_service::configuration::Configuration as ManagementServiceConfiguration;
use paddler::balancer::model_metadata_sender_collection::ModelMetadataSenderCollection;
use paddler::balancer::reconciliation_service::ReconciliationService;
use paddler::balancer::state_database::File;
use paddler::balancer::state_database::Memory;
use paddler::balancer::state_database::StateDatabase;
use paddler::balancer::state_database_type::StateDatabaseType;
#[cfg(feature = "web_admin_panel")]
use paddler::balancer::web_admin_panel_service::configuration::Configuration as WebAdminPanelServiceConfiguration;
use paddler::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use paddler::service_manager::ServiceManager;
use tokio::sync::broadcast;

pub struct Balancer {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub buffered_request_manager: Arc<BufferedRequestManager>,
    pub service_manager: ServiceManager,
    pub state_database: Arc<dyn StateDatabase>,
}

impl Balancer {
    pub async fn bootstrap(
        inference_service_configuration: InferenceServiceConfiguration,
        management_service_configuration: ManagementServiceConfiguration,
        #[cfg(feature = "web_admin_panel")] web_admin_panel_service_configuration: Option<
            WebAdminPanelServiceConfiguration,
        >,
        buffered_request_timeout: Duration,
        max_buffered_requests: i32,
        openai_service_configuration: Option<OpenAIServiceConfiguration>,
        state_database_type: StateDatabaseType,
        statsd_prefix: String,
    ) -> anyhow::Result<Balancer> {
        let (balancer_desired_state_tx, balancer_desired_state_rx) = broadcast::channel(100);

        let agent_controller_pool = Arc::new(AgentControllerPool::default());
        let balancer_applicable_state_holder = Arc::new(BalancerApplicableStateHolder::default());
        let buffered_request_manager = Arc::new(BufferedRequestManager::new(
            agent_controller_pool.clone(),
            buffered_request_timeout,
            max_buffered_requests,
        ));
        let chat_template_override_sender_collection =
            Arc::new(ChatTemplateOverrideSenderCollection::default());
        let embedding_sender_collection = Arc::new(EmbeddingSenderCollection::default());
        let generate_tokens_sender_collection = Arc::new(GenerateTokensSenderCollection::default());
        let model_metadata_sender_collection = Arc::new(ModelMetadataSenderCollection::default());
        let mut service_manager = ServiceManager::default();
        let state_database: Arc<dyn StateDatabase> = match state_database_type {
            StateDatabaseType::File(path) => {
                Arc::new(File::new(balancer_desired_state_tx.clone(), path))
            }
            StateDatabaseType::Memory => Arc::new(Memory::new(balancer_desired_state_tx.clone())),
        };

        service_manager.add_service(InferenceService {
            balancer_applicable_state_holder: balancer_applicable_state_holder.clone(),
            buffered_request_manager: buffered_request_manager.clone(),
            configuration: inference_service_configuration.clone(),
            #[cfg(feature = "web_admin_panel")]
            web_admin_panel_service_configuration: web_admin_panel_service_configuration.clone(),
        });

        service_manager.add_service(ManagementService {
            agent_controller_pool: agent_controller_pool.clone(),
            balancer_applicable_state_holder: balancer_applicable_state_holder.clone(),
            buffered_request_manager: buffered_request_manager.clone(),
            chat_template_override_sender_collection,
            configuration: management_service_configuration,
            embedding_sender_collection,
            generate_tokens_sender_collection,
            model_metadata_sender_collection,
            state_database: state_database.clone(),
            statsd_prefix,
            #[cfg(feature = "web_admin_panel")]
            web_admin_panel_service_configuration,
        });

        service_manager.add_service(ReconciliationService {
            agent_controller_pool: agent_controller_pool.clone(),
            balancer_applicable_state_holder,
            balancer_desired_state: state_database.read_balancer_desired_state().await?,
            balancer_desired_state_rx,
            is_converted_to_applicable_state: false,
        });

        if let Some(openai_configuration) = openai_service_configuration {
            service_manager.add_service(OpenAIService {
                buffered_request_manager: buffered_request_manager.clone(),
                inference_service_configuration,
                openai_service_configuration: openai_configuration,
            });
        }

        Ok(Balancer {
            agent_controller_pool,
            buffered_request_manager,
            service_manager,
            state_database,
        })
    }
}
