use std::io::stdout;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;
use command_handler::handler::Handler;
use tokio_util::sync::CancellationToken;
use trzcina::ServiceShutdownOptions;

use paddler_balancer::compatibility::compatibility_service_configuration::CompatibilityServiceConfiguration;
use paddler_balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use paddler_balancer::management_service::configuration::Configuration as ManagementServiceConfiguration;
use paddler_balancer::resolved_socket_addr::ResolvedSocketAddr;
use paddler_balancer::statsd_service::configuration::Configuration as StatsdServiceConfiguration;
#[cfg(feature = "web_admin_panel")]
use paddler_balancer::web_admin_panel_service::configuration::Configuration as WebAdminPanelServiceConfiguration;
use paddler_balancer_runner::balancer_defaults::BalancerDefaults;
use paddler_balancer_runner::balancer_runner_config::BalancerRunnerConfig;
use paddler_balancer_runner::balancer_service_bundle::BalancerServiceBundle;
use paddler_service_thread::run_service_manager::run_service_manager;
use paddler_state_database::state_database_type::StateDatabaseType;

use super::value_parser::parse_duration::parse_duration;
use super::value_parser::parse_socket_addr::parse_socket_addr;

#[derive(Parser)]
pub struct Balancer {
    #[arg(
        long,
        default_value = BalancerDefaults::BUFFERED_REQUEST_TIMEOUT.as_millis().to_string(),
        value_parser = parse_duration
    )]
    /// Specifies how long a request can stay in the buffer before it is processed.
    /// If the request stays in the buffer longer than this time, it is rejected with the 504 error
    buffered_request_timeout: Duration,

    #[arg(long, value_parser = parse_socket_addr)]
    /// Address of the OpenAI-compatible API server (enabled only if this address is specified)
    compat_openai_addr: Option<ResolvedSocketAddr>,

    #[arg(long, value_parser = parse_socket_addr)]
    /// Address of the TypeSafe-compatible API server (enabled only if this address is specified)
    compat_typesafe_addr: Option<ResolvedSocketAddr>,

    #[arg(
        long,
        default_value = SocketAddr::from((Ipv4Addr::LOCALHOST, BalancerDefaults::INFERENCE_PORT)).to_string(),
        value_parser = parse_socket_addr
    )]
    /// Address of the inference server
    inference_addr: ResolvedSocketAddr,

    #[arg(
        long,
        default_value = BalancerDefaults::INFERENCE_ITEM_TIMEOUT.as_millis().to_string(),
        value_parser = parse_duration
    )]
    /// The timeout (in milliseconds) for generating a single token or a single embedding
    inference_item_timeout: Duration,

    #[arg(
        long = "inference-cors-allowed-host",
        action = clap::ArgAction::Append
    )]
    /// Allowed CORS host for the inference service (can be specified multiple times)
    inference_cors_allowed_hosts: Vec<String>,

    #[arg(
        long,
        default_value = SocketAddr::from((Ipv4Addr::LOCALHOST, BalancerDefaults::MANAGEMENT_PORT)).to_string(),
        value_parser = parse_socket_addr
    )]
    /// This is where you can manage your Paddler setup and the agents connect to
    management_addr: ResolvedSocketAddr,

    #[arg(
        long = "management-cors-allowed-host",
        action = clap::ArgAction::Append
    )]
    /// Allowed CORS host for the management service (can be specified multiple times)
    management_cors_allowed_hosts: Vec<String>,

    #[arg(long, default_value_t = BalancerDefaults::MAX_BUFFERED_REQUESTS)]
    /// The maximum number of buffered requests.
    /// If the buffer is full then new requests are rejected with the 503 error
    max_buffered_requests: u64,

    #[arg(long, default_value = "memory://")]
    /// Balancer state database URL. Supported: memory, memory://, or <file:///path> (optional)
    state_database: StateDatabaseType,

    #[arg(long, value_parser = parse_socket_addr)]
    /// Address for the statsd server to report metrics to (enabled only if this address is specified)
    statsd_addr: Option<ResolvedSocketAddr>,

    #[arg(long, default_value = BalancerDefaults::STATSD_PREFIX)]
    /// Prefix for statsd metrics
    statsd_prefix: String,

    #[arg(
        long,
        default_value = BalancerDefaults::STATSD_REPORTING_INTERVAL.as_millis().to_string(),
        value_parser = parse_duration
    )]
    /// Interval (in milliseconds) at which the balancer will report metrics to statsd
    statsd_reporting_interval: Duration,

    #[cfg(feature = "web_admin_panel")]
    #[arg(long, default_value = None, value_parser = parse_socket_addr)]
    /// Address of the web admin panel (enabled only if this address is specified)
    web_admin_panel_addr: Option<ResolvedSocketAddr>,
}

#[async_trait(?Send)]
impl Handler for Balancer {
    async fn handle(self, shutdown: CancellationToken) -> Result<()> {
        let Self {
            buffered_request_timeout,
            compat_openai_addr,
            compat_typesafe_addr,
            inference_addr,
            inference_item_timeout,
            inference_cors_allowed_hosts,
            management_addr,
            management_cors_allowed_hosts,
            max_buffered_requests,
            state_database,
            statsd_addr,
            statsd_prefix,
            statsd_reporting_interval,
            #[cfg(feature = "web_admin_panel")]
            web_admin_panel_addr,
        } = self;

        let bundle = BalancerServiceBundle::new(BalancerRunnerConfig {
            buffered_request_timeout,
            inference_service_configuration: InferenceServiceConfiguration {
                addr: inference_addr,
                cors_allowed_hosts: inference_cors_allowed_hosts,
                inference_item_timeout,
            },
            management_service_configuration: ManagementServiceConfiguration {
                addr: management_addr,
                cors_allowed_hosts: management_cors_allowed_hosts,
            },
            max_buffered_requests,
            openai_service_configuration: compat_openai_addr
                .map(|addr| CompatibilityServiceConfiguration { addr }),
            state_database_type: state_database,
            statsd_prefix,
            statsd_service_configuration: statsd_addr.map(|statsd_addr| {
                StatsdServiceConfiguration {
                    statsd_addr,
                    statsd_reporting_interval,
                }
            }),
            typesafe_service_configuration: compat_typesafe_addr
                .map(|addr| CompatibilityServiceConfiguration { addr }),
            #[cfg(feature = "web_admin_panel")]
            web_admin_panel_service_configuration: web_admin_panel_addr.map(
                |web_admin_panel_addr| WebAdminPanelServiceConfiguration {
                    addr: web_admin_panel_addr.socket_addr,
                },
            ),
        })
        .await?;

        bundle.addresses.write_json_line(stdout().lock())?;

        run_service_manager(bundle, shutdown, ServiceShutdownOptions::default()).await
    }
}
