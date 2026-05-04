use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;
use paddler::balancer::compatibility::openai_service::configuration::Configuration as OpenAIServiceConfiguration;
use paddler::balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use paddler::balancer::management_service::configuration::Configuration as ManagementServiceConfiguration;
use paddler::balancer::state_database_type::StateDatabaseType;
use paddler::balancer::statsd_service::configuration::Configuration as StatsdServiceConfiguration;
#[cfg(feature = "web_admin_panel")]
use paddler::balancer::web_admin_panel_service::configuration::Configuration as WebAdminPanelServiceConfiguration;
#[cfg(feature = "web_admin_panel")]
use paddler::balancer::web_admin_panel_service::template_data::TemplateData;
use paddler::resolved_socket_addr::ResolvedSocketAddr;
use paddler_bootstrap::balancer_runner::BalancerRunner;
use paddler_bootstrap::balancer_runner::BalancerRunnerParams;
use tokio_util::sync::CancellationToken;

use super::handler::Handler;
use super::value_parser::parse_duration;
use super::value_parser::parse_socket_addr;

#[derive(Parser)]
pub struct Balancer {
    #[arg(long, default_value = "10000", value_parser = parse_duration)]
    /// Specifies how long a request can stay in the buffer before it is processed.
    /// If the request stays in the buffer longer than this time, it is rejected with the 504 error
    buffered_request_timeout: Duration,

    #[arg(long, value_parser = parse_socket_addr)]
    /// Address of the OpenAI-compatible API server (enabled only if this address is specified)
    compat_openai_addr: Option<ResolvedSocketAddr>,

    #[arg(long, default_value = "127.0.0.1:8061", value_parser = parse_socket_addr)]
    /// Address of the inference server
    inference_addr: ResolvedSocketAddr,

    #[arg(long, default_value = "30000", value_parser = parse_duration)]
    /// The timeout (in milliseconds) for generating a single token or a single embedding
    inference_item_timeout: Duration,

    #[arg(
        long = "inference-cors-allowed-host",
        action = clap::ArgAction::Append
    )]
    /// Allowed CORS host for the inference service (can be specified multiple times)
    inference_cors_allowed_hosts: Vec<String>,

    #[arg(long, default_value = "127.0.0.1:8060", value_parser = parse_socket_addr)]
    /// This is where you can manage your Paddler setup and the agents connect to
    management_addr: ResolvedSocketAddr,

    #[arg(
        long = "management-cors-allowed-host",
        action = clap::ArgAction::Append
    )]
    /// Allowed CORS host for the management service (can be specified multiple times)
    management_cors_allowed_hosts: Vec<String>,

    #[arg(long, default_value = "30")]
    /// The maximum number of buffered requests.
    /// If the buffer is full then new requests are rejected with the 503 error
    max_buffered_requests: i32,

    #[arg(long, default_value = "memory://")]
    /// Balancer state database URL. Supported: memory, memory://, or <file:///path> (optional)
    state_database: StateDatabaseType,

    #[arg(long, value_parser = parse_socket_addr)]
    /// Address for the statsd server to report metrics to (enabled only if this address is specified)
    statsd_addr: Option<ResolvedSocketAddr>,

    #[arg(long, default_value = "paddler_")]
    /// Prefix for statsd metrics
    statsd_prefix: String,

    #[arg(long, default_value = "10000", value_parser = parse_duration)]
    /// Interval (in milliseconds) at which the balancer will report metrics to statsd
    statsd_reporting_interval: Duration,

    #[cfg(feature = "web_admin_panel")]
    #[arg(long, default_value = None, value_parser = parse_socket_addr)]
    /// Address of the web admin panel (enabled only if this address is specified)
    web_admin_panel_addr: Option<ResolvedSocketAddr>,
}

impl Balancer {
    #[cfg(feature = "web_admin_panel")]
    fn get_web_admin_panel_service_configuration(
        &self,
    ) -> Option<WebAdminPanelServiceConfiguration> {
        self.web_admin_panel_addr
            .clone()
            .map(|web_admin_panel_addr| WebAdminPanelServiceConfiguration {
                addr: web_admin_panel_addr.socket_addr,
                template_data: TemplateData {
                    buffered_request_timeout: self.buffered_request_timeout,
                    compat_openai_addr: self.compat_openai_addr.clone(),
                    inference_addr: self.inference_addr.clone(),
                    management_addr: self.management_addr.clone(),
                    max_buffered_requests: self.max_buffered_requests,
                    statsd_addr: self.statsd_addr.clone(),
                    statsd_prefix: self.statsd_prefix.clone(),
                    statsd_reporting_interval: self.statsd_reporting_interval,
                },
            })
    }
}

#[async_trait]
impl Handler for Balancer {
    async fn handle(&self, shutdown: CancellationToken) -> Result<()> {
        let mut runner = BalancerRunner::start(BalancerRunnerParams {
            buffered_request_timeout: self.buffered_request_timeout,
            inference_service_configuration: InferenceServiceConfiguration {
                addr: self.inference_addr.socket_addr,
                cors_allowed_hosts: self.inference_cors_allowed_hosts.clone(),
                inference_item_timeout: self.inference_item_timeout,
            },
            management_service_configuration: ManagementServiceConfiguration {
                addr: self.management_addr.socket_addr,
                cors_allowed_hosts: self.management_cors_allowed_hosts.clone(),
            },
            max_buffered_requests: self.max_buffered_requests,
            openai_service_configuration: self.compat_openai_addr.clone().map(
                |compat_openai_addr| OpenAIServiceConfiguration {
                    addr: compat_openai_addr.socket_addr,
                },
            ),
            cancellation_token: shutdown,
            state_database_type: self.state_database.clone(),
            statsd_prefix: self.statsd_prefix.clone(),
            statsd_service_configuration: self.statsd_addr.clone().map(|statsd_addr| {
                StatsdServiceConfiguration {
                    statsd_addr: statsd_addr.socket_addr,
                    statsd_prefix: self.statsd_prefix.clone(),
                    statsd_reporting_interval: self.statsd_reporting_interval,
                }
            }),
            #[cfg(feature = "web_admin_panel")]
            web_admin_panel_service_configuration: self.get_web_admin_panel_service_configuration(),
        })
        .await?;

        runner.wait_for_completion().await
    }
}
