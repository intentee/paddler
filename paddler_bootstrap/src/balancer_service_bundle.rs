use std::sync::Arc;

use trzcina::ServiceManager;

use paddler_balancer::agent_controller_pool::AgentControllerPool;
use paddler_balancer::balancer_addresses::BalancerAddresses;
use paddler_balancer::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use paddler_balancer::balancer_desired_state_store::BalancerDesiredStateStore;
use paddler_balancer::buffered_request_manager::BufferedRequestManager;
use paddler_balancer::compatibility::compatibility_service::CompatibilityService;
use paddler_balancer::compatibility::openai_service::openai_compatibility_layer::OpenAICompatibilityLayer;
use paddler_balancer::compatibility::typesafe_service::typesafe_compatibility_layer::TypeSafeCompatibilityLayer;
use paddler_balancer::http_listener::HttpListener;
use paddler_balancer::inference_service::InferenceService;
use paddler_balancer::management_service::ManagementService;
use paddler_balancer::reconciliation_service::ReconciliationService;
use paddler_balancer::statsd_service::StatsdService;
use paddler_balancer::statsd_service::configuration::Configuration as StatsdServiceConfiguration;
#[cfg(feature = "web_admin_panel")]
use paddler_balancer::web_admin_panel_service::WebAdminPanelService;
#[cfg(feature = "web_admin_panel")]
use paddler_balancer::web_admin_panel_service::configuration::Configuration as WebAdminPanelServiceConfiguration;
#[cfg(feature = "web_admin_panel")]
use paddler_balancer::web_admin_panel_service::template_data::TemplateData;
use paddler_state_database::state_database::StateDatabase;
use paddler_state_database::state_database_type::StateDatabaseType;

use crate::balancer_bootstrap_config::BalancerBootstrapConfig;
use crate::balancer_http_listeners::BalancerHttpListeners;
use crate::bootstrap_error::BootstrapError;
use crate::registers_services::RegistersServices;

fn reject_zero_statsd_reporting_interval(
    statsd_service_configuration: Option<&StatsdServiceConfiguration>,
) -> Result<(), BootstrapError> {
    if statsd_service_configuration.is_some_and(
        |&StatsdServiceConfiguration {
             statsd_reporting_interval,
             ..
         }| statsd_reporting_interval.is_zero(),
    ) {
        return Err(BootstrapError::StatsdReportingIntervalIsZero);
    }

    Ok(())
}

async fn load_balancer_desired_state_store(
    state_database_type: StateDatabaseType,
) -> Result<BalancerDesiredStateStore, BootstrapError> {
    BalancerDesiredStateStore::load(Arc::new(StateDatabase::from(state_database_type)))
        .await
        .map_err(|source| BootstrapError::StateDatabaseReadFailed { source })
}

#[cfg(feature = "web_admin_panel")]
fn bind_web_admin_panel_service(
    web_admin_panel_service_configuration: Option<WebAdminPanelServiceConfiguration>,
    template_data: TemplateData,
) -> Result<Option<WebAdminPanelService>, BootstrapError> {
    web_admin_panel_service_configuration
        .map(|WebAdminPanelServiceConfiguration { addr }| {
            HttpListener::bind(addr)
                .map(|http_listener| WebAdminPanelService {
                    http_listener,
                    template_data,
                })
                .map_err(|source| BootstrapError::WebAdminPanelBindFailed { addr, source })
        })
        .transpose()
}

fn compatibility_service<TCompatibilityLayer>(
    InferenceService {
        agent_controller_pool,
        balancer_applicable_state_holder,
        buffered_request_manager,
        configuration,
        ..
    }: &InferenceService,
    compatibility_layer: TCompatibilityLayer,
    http_listener: HttpListener,
) -> CompatibilityService<TCompatibilityLayer> {
    CompatibilityService {
        agent_controller_pool: agent_controller_pool.clone(),
        balancer_applicable_state_holder: balancer_applicable_state_holder.clone(),
        buffered_request_manager: buffered_request_manager.clone(),
        compatibility_layer,
        http_listener,
        inference_service_configuration: configuration.clone(),
    }
}

pub struct BalancerServiceBundle {
    pub addresses: BalancerAddresses,
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
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
    /// # Errors
    ///
    /// Returns [`BootstrapError::StatsdReportingIntervalIsZero`] when the statsd reporting interval is zero, [`BootstrapError::StateDatabaseReadFailed`] when the balancer desired state cannot be read from the state database, and [`BootstrapError::InferenceBindFailed`], [`BootstrapError::ManagementBindFailed`], [`BootstrapError::CompatOpenAIBindFailed`], [`BootstrapError::CompatTypeSafeBindFailed`] or `BootstrapError::WebAdminPanelBindFailed` when the address of the corresponding service cannot be bound.
    pub async fn new(
        balancer_bootstrap_config: BalancerBootstrapConfig,
    ) -> Result<Self, BootstrapError> {
        #[cfg(feature = "web_admin_panel")]
        let web_admin_panel_template_data =
            balancer_bootstrap_config.web_admin_panel_template_data();
        let BalancerBootstrapConfig {
            buffered_request_timeout,
            inference_service_configuration,
            management_service_configuration,
            max_buffered_requests,
            openai_service_configuration,
            state_database_type,
            statsd_prefix,
            statsd_service_configuration,
            typesafe_service_configuration,
            #[cfg(feature = "web_admin_panel")]
            web_admin_panel_service_configuration,
        } = balancer_bootstrap_config;

        reject_zero_statsd_reporting_interval(statsd_service_configuration.as_ref())?;

        let agent_controller_pool = Arc::new(AgentControllerPool::default());
        let buffered_request_manager = Arc::new(BufferedRequestManager::new(
            agent_controller_pool.clone(),
            buffered_request_timeout,
            max_buffered_requests,
        ));
        let balancer_desired_state_store =
            Arc::new(load_balancer_desired_state_store(state_database_type).await?);
        let balancer_applicable_state_holder =
            Arc::new(balancer_desired_state_store.applicable_state_holder());

        let balancer_http_listeners = BalancerHttpListeners::bind(
            &inference_service_configuration,
            &management_service_configuration,
            openai_service_configuration.as_ref(),
            typesafe_service_configuration.as_ref(),
        )?;
        #[cfg(feature = "web_admin_panel")]
        let web_admin_panel_service = bind_web_admin_panel_service(
            web_admin_panel_service_configuration,
            web_admin_panel_template_data,
        )?;

        let addresses = balancer_http_listeners.addresses();
        #[cfg(feature = "web_admin_panel")]
        let addresses = BalancerAddresses {
            web_admin_panel: web_admin_panel_service
                .as_ref()
                .map(|WebAdminPanelService { http_listener, .. }| http_listener.local_addr),
            ..addresses
        };

        let inference_service = InferenceService {
            agent_controller_pool: agent_controller_pool.clone(),
            balancer_applicable_state_holder: balancer_applicable_state_holder.clone(),
            buffered_request_manager: buffered_request_manager.clone(),
            configuration: inference_service_configuration,
            http_listener: balancer_http_listeners.inference,
            web_admin_panel_addr: addresses.web_admin_panel,
        };

        Ok(Self {
            addresses,
            balancer_applicable_state_holder: balancer_applicable_state_holder.clone(),
            management_service: ManagementService {
                agent_controller_pool: agent_controller_pool.clone(),
                balancer_applicable_state_holder: balancer_applicable_state_holder.clone(),
                buffered_request_manager: buffered_request_manager.clone(),
                configuration: management_service_configuration,
                balancer_desired_state_store,
                http_listener: balancer_http_listeners.management,
                statsd_prefix: statsd_prefix.clone(),
                web_admin_panel_addr: addresses.web_admin_panel,
            },
            reconciliation_service: ReconciliationService {
                agent_controller_pool: agent_controller_pool.clone(),
                balancer_applicable_state_holder,
            },
            openai_service: balancer_http_listeners.compat_openai.map(|http_listener| {
                compatibility_service(&inference_service, OpenAICompatibilityLayer, http_listener)
            }),
            statsd_service: statsd_service_configuration.map(|configuration| StatsdService {
                agent_controller_pool,
                buffered_request_manager,
                configuration,
                statsd_prefix,
            }),
            typesafe_service: balancer_http_listeners
                .compat_typesafe
                .map(|http_listener| {
                    compatibility_service(
                        &inference_service,
                        TypeSafeCompatibilityLayer,
                        http_listener,
                    )
                }),
            inference_service,
            #[cfg(feature = "web_admin_panel")]
            web_admin_panel_service,
        })
    }
}

impl RegistersServices for BalancerServiceBundle {
    fn register_services(self, service_manager: &mut ServiceManager) {
        service_manager.register_service(self.inference_service);
        service_manager.register_service(self.management_service);
        service_manager.register_service(self.reconciliation_service);

        if let Some(service) = self.openai_service {
            service_manager.register_service(service);
        }

        if let Some(service) = self.statsd_service {
            service_manager.register_service(service);
        }

        if let Some(service) = self.typesafe_service {
            service_manager.register_service(service);
        }

        #[cfg(feature = "web_admin_panel")]
        if let Some(service) = self.web_admin_panel_service {
            service_manager.register_service(service);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use paddler_balancer::compatibility::compatibility_service_configuration::CompatibilityServiceConfiguration;
    use paddler_balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
    use paddler_balancer::management_service::configuration::Configuration as ManagementServiceConfiguration;
    use paddler_balancer::resolved_socket_addr::ResolvedSocketAddr;
    use paddler_balancer::statsd_service::configuration::Configuration as StatsdServiceConfiguration;
    #[cfg(feature = "web_admin_panel")]
    use paddler_balancer::web_admin_panel_service::configuration::Configuration as WebAdminPanelServiceConfiguration;
    use paddler_state_database::state_database_type::StateDatabaseType;
    use paddler_state_database::stored_balancer_desired_state::StoredBalancerDesiredState;
    use paddler_test_cluster_harness::constants::EPHEMERAL_LOOPBACK_ADDR;

    use super::BalancerServiceBundle;
    use crate::balancer_bootstrap_config::BalancerBootstrapConfig;

    fn fully_configured_runner_config() -> BalancerBootstrapConfig {
        BalancerBootstrapConfig {
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
            openai_service_configuration: Some(CompatibilityServiceConfiguration {
                addr: ResolvedSocketAddr::from(EPHEMERAL_LOOPBACK_ADDR),
            }),
            state_database_type: StateDatabaseType::Memory(
                StoredBalancerDesiredState::NothingStored,
            ),
            statsd_prefix: "paddler_bootstrap_test_".to_owned(),
            statsd_service_configuration: Some(StatsdServiceConfiguration {
                statsd_addr: ResolvedSocketAddr::from(EPHEMERAL_LOOPBACK_ADDR),
                statsd_reporting_interval: Duration::from_secs(10),
            }),
            typesafe_service_configuration: Some(CompatibilityServiceConfiguration {
                addr: ResolvedSocketAddr::from(EPHEMERAL_LOOPBACK_ADDR),
            }),
            #[cfg(feature = "web_admin_panel")]
            web_admin_panel_service_configuration: Some(WebAdminPanelServiceConfiguration {
                addr: EPHEMERAL_LOOPBACK_ADDR,
            }),
        }
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
            bundle
                .addresses
                .compat_typesafe
                .map(|compat_typesafe| compat_typesafe.port()),
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
}
