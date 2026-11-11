use std::time::Duration;

use paddler_balancer::compatibility::compatibility_service_configuration::CompatibilityServiceConfiguration;
use paddler_balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use paddler_balancer::management_service::configuration::Configuration as ManagementServiceConfiguration;
use paddler_balancer::statsd_service::configuration::Configuration as StatsdServiceConfiguration;
#[cfg(feature = "web_admin_panel")]
use paddler_balancer::web_admin_panel_service::configuration::Configuration as WebAdminPanelServiceConfiguration;
#[cfg(feature = "web_admin_panel")]
use paddler_balancer::web_admin_panel_service::template_data::TemplateData;
use paddler_state_database::state_database_type::StateDatabaseType;

pub struct BalancerBootstrapConfig {
    pub buffered_request_timeout: Duration,
    pub inference_service_configuration: InferenceServiceConfiguration,
    pub management_service_configuration: ManagementServiceConfiguration,
    pub max_buffered_requests: u64,
    pub openai_service_configuration: Option<CompatibilityServiceConfiguration>,
    pub state_database_type: StateDatabaseType,
    pub statsd_prefix: String,
    pub statsd_service_configuration: Option<StatsdServiceConfiguration>,
    pub typesafe_service_configuration: Option<CompatibilityServiceConfiguration>,
    #[cfg(feature = "web_admin_panel")]
    pub web_admin_panel_service_configuration: Option<WebAdminPanelServiceConfiguration>,
}

impl BalancerBootstrapConfig {
    #[cfg(feature = "web_admin_panel")]
    #[must_use]
    pub fn web_admin_panel_template_data(&self) -> TemplateData {
        TemplateData {
            buffered_request_timeout: self.buffered_request_timeout,
            compat_openai_addr: self
                .openai_service_configuration
                .as_ref()
                .map(|CompatibilityServiceConfiguration { addr }| addr.clone()),
            compat_typesafe_addr: self
                .typesafe_service_configuration
                .as_ref()
                .map(|CompatibilityServiceConfiguration { addr }| addr.clone()),
            inference_addr: self.inference_service_configuration.addr.clone(),
            management_addr: self.management_service_configuration.addr.clone(),
            max_buffered_requests: self.max_buffered_requests,
            statsd_prefix: self.statsd_prefix.clone(),
            statsd_service_configuration: self.statsd_service_configuration.clone(),
        }
    }
}
