use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use paddler_balancer::agent_controller_pool::AgentControllerPool;
use paddler_balancer::balancer_addresses::BalancerAddresses;
use paddler_balancer::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use paddler_balancer::buffered_request_manager::BufferedRequestManager;
use paddler_balancer::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
use paddler_balancer::compatibility::openai_service::OpenAIService;
use paddler_balancer::embedding_sender_collection::EmbeddingSenderCollection;
use paddler_balancer::generate_tokens_sender_collection::GenerateTokensSenderCollection;
use paddler_balancer::http_listener::HttpListener;
use paddler_balancer::inference_service::InferenceService;
use paddler_balancer::management_service::ManagementService;
use paddler_balancer::model_metadata_sender_collection::ModelMetadataSenderCollection;
use paddler_balancer::reconciliation_service::ReconciliationService;
use paddler_balancer::state_database::StateDatabase;
use paddler_balancer::state_database::file::File;
use paddler_balancer::state_database::memory::Memory;
use paddler_balancer::state_database_type::StateDatabaseType;
use paddler_balancer::statsd_service::StatsdService;
#[cfg(feature = "web_admin_panel")]
use paddler_balancer::web_admin_panel_service::WebAdminPanelService;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use tokio::sync::broadcast;
use trzcina::Service;
use trzcina::ServiceBundle;

use crate::balancer_bootstrap_config::BalancerBootstrapConfig;
use crate::bootstrap_error::BootstrapError;

pub struct BalancerServiceBundle {
    pub addresses: BalancerAddresses,
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
            state_database_type,
            statsd_prefix,
            statsd_service_configuration,
            #[cfg(feature = "web_admin_panel")]
            web_admin_panel_service_configuration,
        }: BalancerBootstrapConfig,
    ) -> Result<Self, BootstrapError> {
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

        let initial_desired_state = state_database
            .read_balancer_desired_state()
            .await
            .map_err(|source| BootstrapError::StateDatabaseReadFailed { source })?;

        let inference_http_listener = HttpListener::bind(inference_service_configuration.addr)
            .map_err(|source| BootstrapError::InferenceBindFailed {
                addr: inference_service_configuration.addr,
                source,
            })?;
        let management_http_listener = HttpListener::bind(management_service_configuration.addr)
            .map_err(|source| BootstrapError::ManagementBindFailed {
                addr: management_service_configuration.addr,
                source,
            })?;
        let openai_http_listener = openai_service_configuration
            .map(|openai_service_configuration| {
                HttpListener::bind(openai_service_configuration.addr).map_err(|source| {
                    BootstrapError::CompatOpenAIBindFailed {
                        addr: openai_service_configuration.addr,
                        source,
                    }
                })
            })
            .transpose()?;
        #[cfg(feature = "web_admin_panel")]
        let web_admin_panel_service = web_admin_panel_service_configuration
            .map(|configuration| {
                let addr = configuration.addr;

                HttpListener::bind(addr)
                    .map(|http_listener| WebAdminPanelService {
                        configuration,
                        http_listener,
                    })
                    .map_err(|source| BootstrapError::WebAdminPanelBindFailed { addr, source })
            })
            .transpose()?;
        #[cfg(feature = "web_admin_panel")]
        let web_admin_panel_addr = web_admin_panel_service
            .as_ref()
            .map(|web_admin_panel_service| web_admin_panel_service.http_listener.local_addr);
        #[cfg(not(feature = "web_admin_panel"))]
        let web_admin_panel_addr = None;

        let addresses = BalancerAddresses {
            compat_openai: openai_http_listener
                .as_ref()
                .map(|openai_http_listener| openai_http_listener.local_addr),
            inference: inference_http_listener.local_addr,
            management: management_http_listener.local_addr,
            web_admin_panel: web_admin_panel_addr,
        };

        let inference_service = InferenceService {
            agent_controller_pool: agent_controller_pool.clone(),
            balancer_applicable_state_holder: balancer_applicable_state_holder.clone(),
            buffered_request_manager: buffered_request_manager.clone(),
            configuration: inference_service_configuration.clone(),
            http_listener: inference_http_listener,
            web_admin_panel_addr,
        };

        let management_service = ManagementService {
            agent_controller_pool: agent_controller_pool.clone(),
            balancer_applicable_state_holder: balancer_applicable_state_holder.clone(),
            buffered_request_manager: buffered_request_manager.clone(),
            chat_template_override_sender_collection,
            configuration: management_service_configuration,
            embedding_sender_collection,
            generate_tokens_sender_collection,
            http_listener: management_http_listener,
            model_metadata_sender_collection,
            state_database: state_database.clone(),
            statsd_prefix,
            web_admin_panel_addr,
        };

        let reconciliation_service = ReconciliationService {
            agent_controller_pool: agent_controller_pool.clone(),
            balancer_applicable_state_holder: balancer_applicable_state_holder.clone(),
            balancer_desired_state: initial_desired_state.clone(),
            balancer_desired_state_rx,
            is_converted_to_applicable_state: false,
        };

        let openai_service = openai_http_listener.map(|http_listener| OpenAIService {
            balancer_applicable_state_holder: balancer_applicable_state_holder.clone(),
            buffered_request_manager: buffered_request_manager.clone(),
            http_listener,
            inference_service_configuration,
        });

        let statsd_service = statsd_service_configuration.map(|configuration| StatsdService {
            agent_controller_pool: agent_controller_pool.clone(),
            buffered_request_manager,
            configuration,
        });

        Ok(Self {
            addresses,
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
    use std::net::IpAddr;
    use std::net::Ipv4Addr;
    use std::net::SocketAddr;
    use std::net::TcpListener;
    use std::time::Duration;

    use paddler_balancer::compatibility::openai_service::configuration::Configuration as OpenAIServiceConfiguration;
    use paddler_balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
    use paddler_balancer::management_service::configuration::Configuration as ManagementServiceConfiguration;
    #[cfg(feature = "web_admin_panel")]
    use paddler_balancer::resolved_socket_addr::ResolvedSocketAddr;
    use paddler_balancer::state_database_type::StateDatabaseType;
    use paddler_balancer::statsd_service::configuration::Configuration as StatsdServiceConfiguration;
    #[cfg(feature = "web_admin_panel")]
    use paddler_balancer::web_admin_panel_service::configuration::Configuration as WebAdminPanelServiceConfiguration;
    #[cfg(feature = "web_admin_panel")]
    use paddler_balancer::web_admin_panel_service::template_data::TemplateData;
    use trzcina::ServiceBundle as _;

    use super::BalancerServiceBundle;
    use crate::balancer_bootstrap_config::BalancerBootstrapConfig;
    use crate::bootstrap_error::BootstrapError;

    #[cfg(feature = "web_admin_panel")]
    const EXPECTED_SERVICE_COUNT: usize = 6;
    #[cfg(not(feature = "web_admin_panel"))]
    const EXPECTED_SERVICE_COUNT: usize = 5;

    const EPHEMERAL_LOOPBACK_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);

    fn occupy_ephemeral_address() -> TcpListener {
        TcpListener::bind(EPHEMERAL_LOOPBACK_ADDR)
            .expect("an ephemeral loopback port must be bindable")
    }

    fn address_of(occupying_listener: &TcpListener) -> SocketAddr {
        occupying_listener
            .local_addr()
            .expect("a bound listener must report its address")
    }

    fn fully_configured_bootstrap_config() -> BalancerBootstrapConfig {
        BalancerBootstrapConfig {
            buffered_request_timeout: Duration::from_secs(10),
            inference_service_configuration: InferenceServiceConfiguration {
                addr: EPHEMERAL_LOOPBACK_ADDR,
                cors_allowed_hosts: vec![],
                inference_item_timeout: Duration::from_secs(30),
            },
            management_service_configuration: ManagementServiceConfiguration {
                addr: EPHEMERAL_LOOPBACK_ADDR,
                cors_allowed_hosts: vec![],
            },
            max_buffered_requests: 30,
            openai_service_configuration: Some(OpenAIServiceConfiguration {
                addr: EPHEMERAL_LOOPBACK_ADDR,
            }),
            state_database_type: StateDatabaseType::Memory(Box::default()),
            statsd_prefix: "paddler_bootstrap_test_".to_owned(),
            statsd_service_configuration: Some(StatsdServiceConfiguration {
                statsd_addr: EPHEMERAL_LOOPBACK_ADDR,
                statsd_prefix: "paddler_bootstrap_test_".to_owned(),
                statsd_reporting_interval: Duration::from_secs(10),
            }),
            #[cfg(feature = "web_admin_panel")]
            web_admin_panel_service_configuration: Some(WebAdminPanelServiceConfiguration {
                addr: EPHEMERAL_LOOPBACK_ADDR,
                template_data: TemplateData {
                    buffered_request_timeout: Duration::from_secs(10),
                    compat_openai_addr: None,
                    inference_addr: ResolvedSocketAddr {
                        input_addr: "127.0.0.1:0".to_owned(),
                        socket_addr: EPHEMERAL_LOOPBACK_ADDR,
                    },
                    management_addr: ResolvedSocketAddr {
                        input_addr: "127.0.0.1:0".to_owned(),
                        socket_addr: EPHEMERAL_LOOPBACK_ADDR,
                    },
                    max_buffered_requests: 30,
                    statsd_addr: None,
                    statsd_prefix: "paddler_bootstrap_test_".to_owned(),
                    statsd_reporting_interval: Duration::from_secs(10),
                },
            }),
        }
    }

    async fn bootstrap_error_of(config: BalancerBootstrapConfig) -> BootstrapError {
        BalancerServiceBundle::new(config)
            .await
            .err()
            .expect("the bundle must fail to bind an occupied address")
    }

    #[tokio::test]
    async fn services_includes_every_optional_service_when_configured() {
        let bundle = BalancerServiceBundle::new(fully_configured_bootstrap_config())
            .await
            .expect("a fully configured bundle must bind its ephemeral addresses");

        let services = bundle
            .services()
            .await
            .expect("a bound bundle must produce its services");

        assert_eq!(services.len(), EXPECTED_SERVICE_COUNT);
    }

    #[tokio::test]
    async fn reports_the_bound_address_of_every_configured_service() {
        let bundle = BalancerServiceBundle::new(fully_configured_bootstrap_config())
            .await
            .expect("a fully configured bundle must bind its ephemeral addresses");

        let bound_ports = [
            bundle
                .addresses
                .compat_openai
                .map(|compat_openai| compat_openai.port()),
            Some(bundle.addresses.inference.port()),
            Some(bundle.addresses.management.port()),
            #[cfg(feature = "web_admin_panel")]
            bundle
                .addresses
                .web_admin_panel
                .map(|web_admin_panel| web_admin_panel.port()),
        ];

        assert!(
            bound_ports
                .iter()
                .all(|bound_port| bound_port.is_some_and(|port| port != 0))
        );
    }

    #[tokio::test]
    async fn fails_when_the_management_address_is_taken() {
        let occupying_listener = occupy_ephemeral_address();
        let taken_addr = address_of(&occupying_listener);
        let mut config = fully_configured_bootstrap_config();

        config.management_service_configuration.addr = taken_addr;

        assert!(matches!(
            bootstrap_error_of(config).await,
            BootstrapError::ManagementBindFailed { addr, .. } if addr == taken_addr
        ));
    }

    #[tokio::test]
    async fn fails_when_the_compat_openai_address_is_taken() {
        let occupying_listener = occupy_ephemeral_address();
        let taken_addr = address_of(&occupying_listener);
        let mut config = fully_configured_bootstrap_config();

        config.openai_service_configuration = Some(OpenAIServiceConfiguration { addr: taken_addr });

        assert!(matches!(
            bootstrap_error_of(config).await,
            BootstrapError::CompatOpenAIBindFailed { addr, .. } if addr == taken_addr
        ));
    }

    #[cfg(feature = "web_admin_panel")]
    #[tokio::test]
    async fn fails_when_the_web_admin_panel_address_is_taken() {
        let occupying_listener = occupy_ephemeral_address();
        let taken_addr = address_of(&occupying_listener);
        let mut config = fully_configured_bootstrap_config();

        config
            .web_admin_panel_service_configuration
            .as_mut()
            .expect("the fully configured bootstrap config must include the web admin panel")
            .addr = taken_addr;

        assert!(matches!(
            bootstrap_error_of(config).await,
            BootstrapError::WebAdminPanelBindFailed { addr, .. } if addr == taken_addr
        ));
    }
}
