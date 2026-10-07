use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::watch;
use trzcina::Service;
use trzcina::ServiceBundle;

use paddler_balancer::agent_controller_pool::AgentControllerPool;
use paddler_balancer::agent_response_senders::AgentResponseSenders;
use paddler_balancer::balancer_addresses::BalancerAddresses;
use paddler_balancer::balancer_applicable_state::BalancerApplicableState;
use paddler_balancer::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use paddler_balancer::buffered_request_manager::BufferedRequestManager;
use paddler_balancer::compatibility::compatibility_service::CompatibilityService;
#[cfg(feature = "web_admin_panel")]
use paddler_balancer::compatibility::compatibility_service_configuration::CompatibilityServiceConfiguration;
use paddler_balancer::compatibility::openai_service::openai_compatibility_layer::OpenAICompatibilityLayer;
use paddler_balancer::compatibility::typesafe_service::typesafe_compatibility_layer::TypeSafeCompatibilityLayer;
use paddler_balancer::http_listener::HttpListener;
use paddler_balancer::inference_service::InferenceService;
use paddler_balancer::management_service::ManagementService;
use paddler_balancer::reconciliation_service::ReconciliationService;
use paddler_balancer::statsd_service::StatsdService;
#[cfg(feature = "web_admin_panel")]
use paddler_balancer::web_admin_panel_service::WebAdminPanelService;
#[cfg(feature = "web_admin_panel")]
use paddler_balancer::web_admin_panel_service::configuration::Configuration as WebAdminPanelServiceConfiguration;
#[cfg(feature = "web_admin_panel")]
use paddler_balancer::web_admin_panel_service::template_data::TemplateData;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_state_database::file::File;
use paddler_state_database::memory::Memory;
use paddler_state_database::state_database::StateDatabase;
use paddler_state_database::state_database_type::StateDatabaseType;

use crate::balancer_runner_config::BalancerRunnerConfig;
use crate::balancer_runner_error::BalancerRunnerError;

pub struct BalancerServiceBundle {
    pub addresses: BalancerAddresses,
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    pub balancer_desired_state_tx: watch::Sender<BalancerDesiredState>,
    pub state_database: Arc<dyn StateDatabase>,
    inference_service: InferenceService,
    management_service: ManagementService,
    reconciliation_service: ReconciliationService,
    openai_service: Option<CompatibilityService<OpenAICompatibilityLayer>>,
    statsd_service: Option<StatsdService>,
    typesafe_service: Option<CompatibilityService<TypeSafeCompatibilityLayer>>,
    #[cfg(feature = "web_admin_panel")]
    web_admin_panel_service: Option<WebAdminPanelService>,
}

impl BalancerServiceBundle {
    pub async fn new(
        BalancerRunnerConfig {
            buffered_request_timeout,
            inference_service_configuration,
            management_service_configuration,
            max_buffered_requests,
            serving_mode,
            state_database_type,
            statsd_prefix,
            statsd_service_configuration,
            #[cfg(feature = "web_admin_panel")]
            web_admin_panel_service_configuration,
        }: BalancerRunnerConfig,
    ) -> Result<Self, BalancerRunnerError> {
        if statsd_service_configuration
            .as_ref()
            .is_some_and(|statsd_service_configuration| {
                statsd_service_configuration
                    .statsd_reporting_interval
                    .is_zero()
            })
        {
            return Err(BalancerRunnerError::StatsdReportingIntervalIsZero);
        }

        let inference_mode = serving_mode.inference_mode();
        let openai_service_configuration = serving_mode.openai_service_configuration();
        let typesafe_service_configuration = serving_mode.typesafe_service_configuration();
        let (balancer_desired_state_tx, _initial_desired_state_rx) =
            watch::channel(BalancerDesiredState::unconfigured(inference_mode));

        let agent_controller_pool = Arc::new(AgentControllerPool::new(inference_mode));
        let buffered_request_manager = Arc::new(BufferedRequestManager::new(
            agent_controller_pool.clone(),
            buffered_request_timeout,
            max_buffered_requests,
        ));
        let state_database: Arc<dyn StateDatabase> = match state_database_type {
            StateDatabaseType::File(path) => Arc::new(File::new(
                balancer_desired_state_tx.clone(),
                inference_mode,
                path,
            )),
            StateDatabaseType::Memory => Arc::new(Memory::new(
                balancer_desired_state_tx.clone(),
                inference_mode,
                BalancerDesiredState::unconfigured(inference_mode),
            )),
            StateDatabaseType::MemoryStartingWith(initial_desired_state) => Arc::new(Memory::new(
                balancer_desired_state_tx.clone(),
                inference_mode,
                *initial_desired_state,
            )),
        };

        let initial_desired_state = state_database
            .read_balancer_desired_state()
            .await
            .map_err(|source| BalancerRunnerError::StateDatabaseReadFailed { source })?;

        balancer_desired_state_tx.send_replace(initial_desired_state.clone());

        let balancer_applicable_state_holder = Arc::new(BalancerApplicableStateHolder::new(
            BalancerApplicableState::from(initial_desired_state),
        ));

        let inference_addr = inference_service_configuration.addr.socket_addr;
        let inference_http_listener = HttpListener::bind(inference_addr).map_err(|source| {
            BalancerRunnerError::InferenceBindFailed {
                addr: inference_addr,
                source,
            }
        })?;
        let management_addr = management_service_configuration.addr.socket_addr;
        let management_http_listener = HttpListener::bind(management_addr).map_err(|source| {
            BalancerRunnerError::ManagementBindFailed {
                addr: management_addr,
                source,
            }
        })?;
        let openai_http_listener = openai_service_configuration
            .as_ref()
            .map(|openai_service_configuration| {
                let openai_addr = openai_service_configuration.addr.socket_addr;

                HttpListener::bind(openai_addr).map_err(|source| {
                    BalancerRunnerError::CompatOpenAIBindFailed {
                        addr: openai_addr,
                        source,
                    }
                })
            })
            .transpose()?;
        let typesafe_http_listener = typesafe_service_configuration
            .as_ref()
            .map(|typesafe_service_configuration| {
                let typesafe_addr = typesafe_service_configuration.addr.socket_addr;

                HttpListener::bind(typesafe_addr).map_err(|source| {
                    BalancerRunnerError::CompatTypeSafeBindFailed {
                        addr: typesafe_addr,
                        source,
                    }
                })
            })
            .transpose()?;
        #[cfg(feature = "web_admin_panel")]
        let web_admin_panel_service = web_admin_panel_service_configuration
            .map(|WebAdminPanelServiceConfiguration { addr }| {
                HttpListener::bind(addr)
                    .map(|http_listener| WebAdminPanelService {
                        http_listener,
                        template_data: TemplateData {
                            buffered_request_timeout,
                            compat_openai_addr: openai_service_configuration
                                .map(|CompatibilityServiceConfiguration { addr }| addr),
                            compat_typesafe_addr: typesafe_service_configuration
                                .map(|CompatibilityServiceConfiguration { addr }| addr),
                            inference_addr: inference_service_configuration.addr.clone(),
                            inference_mode,
                            management_addr: management_service_configuration.addr.clone(),
                            max_buffered_requests,
                            statsd_prefix: statsd_prefix.clone(),
                            statsd_service_configuration: statsd_service_configuration.clone(),
                        },
                    })
                    .map_err(|source| BalancerRunnerError::WebAdminPanelBindFailed { addr, source })
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
            compat_typesafe: typesafe_http_listener
                .as_ref()
                .map(|typesafe_http_listener| typesafe_http_listener.local_addr),
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
            inference_mode,
            web_admin_panel_addr,
        };

        let management_service = ManagementService {
            agent_controller_pool: agent_controller_pool.clone(),
            balancer_applicable_state_holder: balancer_applicable_state_holder.clone(),
            agent_response_senders: AgentResponseSenders::default(),
            buffered_request_manager: buffered_request_manager.clone(),
            configuration: management_service_configuration,
            http_listener: management_http_listener,
            state_database: state_database.clone(),
            statsd_prefix: statsd_prefix.clone(),
            web_admin_panel_addr,
        };

        let reconciliation_service = ReconciliationService {
            agent_controller_pool: agent_controller_pool.clone(),
            balancer_applicable_state_holder: balancer_applicable_state_holder.clone(),
            balancer_desired_state_rx: balancer_desired_state_tx.subscribe(),
        };

        let openai_service = openai_http_listener.map(|http_listener| CompatibilityService {
            balancer_applicable_state_holder: balancer_applicable_state_holder.clone(),
            buffered_request_manager: buffered_request_manager.clone(),
            compatibility_layer: OpenAICompatibilityLayer,
            http_listener,
            inference_service_configuration: inference_service_configuration.clone(),
        });

        let typesafe_service = typesafe_http_listener.map(|http_listener| CompatibilityService {
            balancer_applicable_state_holder: balancer_applicable_state_holder.clone(),
            buffered_request_manager: buffered_request_manager.clone(),
            compatibility_layer: TypeSafeCompatibilityLayer,
            http_listener,
            inference_service_configuration,
        });

        let statsd_service = statsd_service_configuration.map(|configuration| StatsdService {
            agent_controller_pool: agent_controller_pool.clone(),
            buffered_request_manager,
            configuration,
            statsd_prefix,
        });

        Ok(Self {
            addresses,
            agent_controller_pool,
            balancer_applicable_state_holder,
            balancer_desired_state_tx,
            state_database,
            inference_service,
            management_service,
            reconciliation_service,
            openai_service,
            statsd_service,
            typesafe_service,
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

        if let Some(service) = self.typesafe_service {
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
    use std::time::Duration;

    use trzcina::ServiceBundle as _;

    use paddler_balancer::compatibility::compatibility_service_configuration::CompatibilityServiceConfiguration;
    use paddler_balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
    use paddler_balancer::management_service::configuration::Configuration as ManagementServiceConfiguration;
    use paddler_balancer::resolved_socket_addr::ResolvedSocketAddr;
    use paddler_balancer::statsd_service::configuration::Configuration as StatsdServiceConfiguration;
    #[cfg(feature = "web_admin_panel")]
    use paddler_balancer::web_admin_panel_service::configuration::Configuration as WebAdminPanelServiceConfiguration;
    use paddler_state_database::state_database_type::StateDatabaseType;
    use paddler_test_cluster_harness::ephemeral_loopback_addr::EPHEMERAL_LOOPBACK_ADDR;

    use super::BalancerServiceBundle;
    use crate::balancer_runner_config::BalancerRunnerConfig;
    use crate::balancer_serving_mode::BalancerServingMode;

    #[cfg(feature = "web_admin_panel")]
    const EXPECTED_SERVICE_COUNT: usize = 6;
    #[cfg(not(feature = "web_admin_panel"))]
    const EXPECTED_SERVICE_COUNT: usize = 5;

    fn fully_configured_runner_config() -> BalancerRunnerConfig {
        BalancerRunnerConfig {
            buffered_request_timeout: Duration::from_secs(10),
            inference_service_configuration: InferenceServiceConfiguration {
                addr: ResolvedSocketAddr::from(EPHEMERAL_LOOPBACK_ADDR),
                cors_allowed_hosts: vec![],
                inference_item_timeout: Duration::from_secs(30),
            },
            management_service_configuration: ManagementServiceConfiguration {
                addr: ResolvedSocketAddr::from(EPHEMERAL_LOOPBACK_ADDR),
                cors_allowed_hosts: vec![],
            },
            max_buffered_requests: 30,
            serving_mode: BalancerServingMode::TextGeneration {
                openai_service_configuration: Some(CompatibilityServiceConfiguration {
                    addr: ResolvedSocketAddr::from(EPHEMERAL_LOOPBACK_ADDR),
                }),
            },
            state_database_type: StateDatabaseType::Memory,
            statsd_prefix: "paddler_balancer_runner_test_".to_owned(),
            statsd_service_configuration: Some(StatsdServiceConfiguration {
                statsd_addr: ResolvedSocketAddr::from(EPHEMERAL_LOOPBACK_ADDR),
                statsd_reporting_interval: Duration::from_secs(10),
            }),
            #[cfg(feature = "web_admin_panel")]
            web_admin_panel_service_configuration: Some(WebAdminPanelServiceConfiguration {
                addr: EPHEMERAL_LOOPBACK_ADDR,
            }),
        }
    }

    #[tokio::test]
    async fn services_includes_every_optional_service_when_configured() {
        let bundle = BalancerServiceBundle::new(fully_configured_runner_config())
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
        let bundle = BalancerServiceBundle::new(fully_configured_runner_config())
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
    async fn reports_the_bound_address_of_the_typesafe_service() {
        let bundle = BalancerServiceBundle::new(BalancerRunnerConfig {
            serving_mode: BalancerServingMode::Decision {
                typesafe_service_configuration: Some(CompatibilityServiceConfiguration {
                    addr: ResolvedSocketAddr::from(EPHEMERAL_LOOPBACK_ADDR),
                }),
            },
            ..fully_configured_runner_config()
        })
        .await
        .expect("a decision bundle must bind its ephemeral addresses");

        assert!(
            bundle
                .addresses
                .compat_typesafe
                .is_some_and(|compat_typesafe| compat_typesafe.port() != 0)
        );
    }
}
