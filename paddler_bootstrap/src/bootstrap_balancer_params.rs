use std::time::Duration;

use paddler::balancer::compatibility::openai_service::configuration::Configuration as OpenAIServiceConfiguration;
use paddler::balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use paddler::balancer::management_service::configuration::Configuration as ManagementServiceConfiguration;
use paddler::balancer::state_database_type::StateDatabaseType;
use paddler::balancer::statsd_service::configuration::Configuration as StatsdServiceConfiguration;
#[cfg(feature = "web_admin_panel")]
use paddler::balancer::web_admin_panel_service::configuration::Configuration as WebAdminPanelServiceConfiguration;

pub struct BootstrapBalancerParams {
    pub buffered_request_timeout: Duration,
    pub inference_service_configuration: InferenceServiceConfiguration,
    pub management_service_configuration: ManagementServiceConfiguration,
    pub max_buffered_requests: i32,
    pub openai_service_configuration: Option<OpenAIServiceConfiguration>,
    pub state_database_type: StateDatabaseType,
    pub statsd_prefix: String,
    pub statsd_service_configuration: Option<StatsdServiceConfiguration>,
    #[cfg(feature = "web_admin_panel")]
    pub web_admin_panel_service_configuration: Option<WebAdminPanelServiceConfiguration>,
}
