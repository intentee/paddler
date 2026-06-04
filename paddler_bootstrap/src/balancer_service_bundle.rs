use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use paddler_balancer::agent_controller_pool::AgentControllerPool;
use paddler_balancer::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use paddler_balancer::buffered_request_manager::BufferedRequestManager;
use paddler_balancer::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
use paddler_balancer::compatibility::openai_service::OpenAIService;
use paddler_balancer::compatibility::openai_service::configuration::Configuration as OpenAIServiceConfiguration;
use paddler_balancer::embedding_sender_collection::EmbeddingSenderCollection;
use paddler_balancer::generate_tokens_sender_collection::GenerateTokensSenderCollection;
use paddler_balancer::inference_service::InferenceService;
use paddler_balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use paddler_balancer::management_service::ManagementService;
use paddler_balancer::management_service::configuration::Configuration as ManagementServiceConfiguration;
use paddler_balancer::model_metadata_sender_collection::ModelMetadataSenderCollection;
use paddler_balancer::reconciliation_service::ReconciliationService;
use paddler_balancer::state_database::StateDatabase;
use paddler_balancer::state_database::file::File;
use paddler_balancer::state_database::memory::Memory;
use paddler_balancer::state_database_type::StateDatabaseType;
use paddler_balancer::statsd_service::StatsdService;
use paddler_balancer::statsd_service::configuration::Configuration as StatsdServiceConfiguration;
#[cfg(feature = "web_admin_panel")]
use paddler_balancer::web_admin_panel_service::WebAdminPanelService;
#[cfg(feature = "web_admin_panel")]
use paddler_balancer::web_admin_panel_service::configuration::Configuration as WebAdminPanelServiceConfiguration;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use tokio::sync::broadcast;
use trzcina::Service;
use trzcina::ServiceBundle;
use trzcina::ServiceShutdownOptions;

pub struct BalancerBootstrapConfig {
    pub buffered_request_timeout: Duration,
    pub inference_service_configuration: InferenceServiceConfiguration,
    pub management_service_configuration: ManagementServiceConfiguration,
    pub max_buffered_requests: i32,
    pub openai_service_configuration: Option<OpenAIServiceConfiguration>,
    pub shutdown_options: ServiceShutdownOptions,
    pub state_database_type: StateDatabaseType,
    pub statsd_prefix: String,
    pub statsd_service_configuration: Option<StatsdServiceConfiguration>,
    #[cfg(feature = "web_admin_panel")]
    pub web_admin_panel_service_configuration: Option<WebAdminPanelServiceConfiguration>,
}

pub struct BalancerServiceBundle {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    pub balancer_desired_state_tx: broadcast::Sender<BalancerDesiredState>,
    pub initial_desired_state: BalancerDesiredState,
    pub state_database: Arc<dyn StateDatabase>,
    inference_service: InferenceService,
    management_service: ManagementService,
    reconciliation_service: ReconciliationService,
    openai_service: Option<OpenAIService>,
    statsd_service: Option<StatsdService>,
    #[cfg(feature = "web_admin_panel")]
    web_admin_panel_service: Option<WebAdminPanelService>,
}

impl BalancerServiceBundle {
    pub async fn new(
        BalancerBootstrapConfig {
            buffered_request_timeout,
            inference_service_configuration,
            management_service_configuration,
            max_buffered_requests,
            openai_service_configuration,
            shutdown_options,
            state_database_type,
            statsd_prefix,
            statsd_service_configuration,
            #[cfg(feature = "web_admin_panel")]
            web_admin_panel_service_configuration,
        }: BalancerBootstrapConfig,
    ) -> Result<Self> {
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
        let state_database: Arc<dyn StateDatabase> = match state_database_type {
            StateDatabaseType::File(path) => {
                Arc::new(File::new(balancer_desired_state_tx.clone(), path))
            }
            StateDatabaseType::Memory(initial_desired_state) => Arc::new(Memory::new(
                balancer_desired_state_tx.clone(),
                *initial_desired_state,
            )),
        };

        let initial_desired_state = state_database.read_balancer_desired_state().await?;

        let inference_service = InferenceService {
            agent_controller_pool: agent_controller_pool.clone(),
            balancer_applicable_state_holder: balancer_applicable_state_holder.clone(),
            buffered_request_manager: buffered_request_manager.clone(),
            configuration: inference_service_configuration.clone(),
            shutdown_options: shutdown_options.clone(),
            #[cfg(feature = "web_admin_panel")]
            web_admin_panel_service_configuration: web_admin_panel_service_configuration.clone(),
        };

        let management_service = ManagementService {
            agent_controller_pool: agent_controller_pool.clone(),
            balancer_applicable_state_holder: balancer_applicable_state_holder.clone(),
            buffered_request_manager: buffered_request_manager.clone(),
            chat_template_override_sender_collection,
            configuration: management_service_configuration,
            embedding_sender_collection,
            generate_tokens_sender_collection,
            model_metadata_sender_collection,
            shutdown_options: shutdown_options.clone(),
            state_database: state_database.clone(),
            statsd_prefix,
            #[cfg(feature = "web_admin_panel")]
            web_admin_panel_service_configuration: web_admin_panel_service_configuration.clone(),
        };

        let reconciliation_service = ReconciliationService {
            agent_controller_pool: agent_controller_pool.clone(),
            balancer_applicable_state_holder: balancer_applicable_state_holder.clone(),
            balancer_desired_state: initial_desired_state.clone(),
            balancer_desired_state_rx,
            is_converted_to_applicable_state: false,
        };

        let openai_service =
            openai_service_configuration.map(|openai_service_configuration| OpenAIService {
                buffered_request_manager: buffered_request_manager.clone(),
                inference_service_configuration,
                openai_service_configuration,
                shutdown_options: shutdown_options.clone(),
            });

        let statsd_service = statsd_service_configuration.map(|configuration| StatsdService {
            agent_controller_pool: agent_controller_pool.clone(),
            buffered_request_manager,
            configuration,
        });

        #[cfg(feature = "web_admin_panel")]
        let web_admin_panel_service =
            web_admin_panel_service_configuration.map(|configuration| WebAdminPanelService {
                configuration,
                shutdown_options: shutdown_options.clone(),
            });

        Ok(Self {
            agent_controller_pool,
            balancer_applicable_state_holder,
            balancer_desired_state_tx,
            initial_desired_state,
            state_database,
            inference_service,
            management_service,
            reconciliation_service,
            openai_service,
            statsd_service,
            #[cfg(feature = "web_admin_panel")]
            web_admin_panel_service,
        })
    }
}

#[async_trait]
impl ServiceBundle for BalancerServiceBundle {
    async fn services(self) -> Result<Vec<Box<dyn Service>>> {
        let mut services: Vec<Box<dyn Service>> = vec![
            Box::new(self.inference_service),
            Box::new(self.management_service),
            Box::new(self.reconciliation_service),
        ];

        if let Some(service) = self.openai_service {
            services.push(Box::new(service));
        }

        if let Some(service) = self.statsd_service {
            services.push(Box::new(service));
        }

        #[cfg(feature = "web_admin_panel")]
        if let Some(service) = self.web_admin_panel_service {
            services.push(Box::new(service));
        }

        Ok(services)
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    #[cfg(feature = "web_admin_panel")]
    use paddler_balancer::resolved_socket_addr::ResolvedSocketAddr;
    #[cfg(feature = "web_admin_panel")]
    use paddler_balancer::web_admin_panel_service::template_data::TemplateData;

    use super::*;

    #[cfg(feature = "web_admin_panel")]
    const EXPECTED_SERVICE_COUNT: usize = 6;
    #[cfg(not(feature = "web_admin_panel"))]
    const EXPECTED_SERVICE_COUNT: usize = 5;

    fn loopback_addr() -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], 0))
    }

    #[tokio::test]
    async fn services_includes_every_optional_service_when_configured() {
        let bundle = BalancerServiceBundle::new(BalancerBootstrapConfig {
            buffered_request_timeout: Duration::from_secs(10),
            inference_service_configuration: InferenceServiceConfiguration {
                addr: loopback_addr(),
                cors_allowed_hosts: vec![],
                inference_item_timeout: Duration::from_secs(30),
            },
            management_service_configuration: ManagementServiceConfiguration {
                addr: loopback_addr(),
                cors_allowed_hosts: vec![],
            },
            max_buffered_requests: 30,
            openai_service_configuration: Some(OpenAIServiceConfiguration {
                addr: loopback_addr(),
            }),
            shutdown_options: ServiceShutdownOptions::default(),
            state_database_type: StateDatabaseType::Memory(Box::default()),
            statsd_prefix: "paddler_bootstrap_test_".to_owned(),
            statsd_service_configuration: Some(StatsdServiceConfiguration {
                statsd_addr: loopback_addr(),
                statsd_prefix: "paddler_bootstrap_test_".to_owned(),
                statsd_reporting_interval: Duration::from_secs(10),
            }),
            #[cfg(feature = "web_admin_panel")]
            web_admin_panel_service_configuration: Some(WebAdminPanelServiceConfiguration {
                addr: loopback_addr(),
                template_data: TemplateData {
                    buffered_request_timeout: Duration::from_secs(10),
                    compat_openai_addr: None,
                    inference_addr: ResolvedSocketAddr {
                        input_addr: "127.0.0.1:0".to_owned(),
                        socket_addr: loopback_addr(),
                    },
                    management_addr: ResolvedSocketAddr {
                        input_addr: "127.0.0.1:0".to_owned(),
                        socket_addr: loopback_addr(),
                    },
                    max_buffered_requests: 30,
                    statsd_addr: None,
                    statsd_prefix: "paddler_bootstrap_test_".to_owned(),
                    statsd_reporting_interval: Duration::from_secs(10),
                },
            }),
        })
        .await
        .unwrap();

        let services = bundle.services().await.unwrap();

        assert_eq!(services.len(), EXPECTED_SERVICE_COUNT);
    }
}
