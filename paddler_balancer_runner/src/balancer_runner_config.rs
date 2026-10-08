use std::time::Duration;

use paddler_balancer::compatibility::compatibility_service_configuration::CompatibilityServiceConfiguration;
use paddler_balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use paddler_balancer::management_service::configuration::Configuration as ManagementServiceConfiguration;
use paddler_balancer::statsd_service::configuration::Configuration as StatsdServiceConfiguration;
#[cfg(feature = "web_admin_panel")]
use paddler_balancer::web_admin_panel_service::configuration::Configuration as WebAdminPanelServiceConfiguration;
use paddler_state_database::state_database_type::StateDatabaseType;

pub struct BalancerRunnerConfig {
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
