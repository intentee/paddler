mod agent;
mod balancer;
mod cmd;
mod llamacpp;
#[cfg(feature = "web_dashboard")]
mod static_files;
mod supervisor;

use std::net::SocketAddr;
use std::net::ToSocketAddrs;
use std::time::Duration;

use anyhow::anyhow;
use anyhow::Result;
use clap::Parser;
use clap::Subcommand;
#[cfg(feature = "web_dashboard")]
use esbuild_metafile::instance::initialize_instance;

#[cfg(feature = "web_dashboard")]
pub const ESBUILD_META_CONTENTS: &str = include_str!("../esbuild-meta.json");

fn resolve_socket_addr(s: &str) -> Result<SocketAddr> {
    let addrs: Vec<SocketAddr> = s.to_socket_addrs()?.collect();

    for addr in &addrs {
        if addr.is_ipv4() {
            return Ok(*addr);
        }
    }

    for addr in addrs {
        if addr.is_ipv6() {
            return Ok(addr);
        }
    }

    Err(anyhow!("Failed to resolve socket address"))
}

fn parse_duration(arg: &str) -> Result<Duration> {
    let milliseconds = arg.parse()?;

    Ok(std::time::Duration::from_millis(milliseconds))
}

fn parse_socket_addr(arg: &str) -> Result<SocketAddr> {
    match arg.parse() {
        Ok(socketaddr) => Ok(socketaddr),
        Err(_) => Ok(resolve_socket_addr(arg)?),
    }
}

#[derive(Parser)]
#[command(arg_required_else_help(true), version, about, long_about = None)]
/// Stateful load balancer for llama.cpp
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Monitors llama.cpp instance and reports their status to the balancer
    Agent {
        #[arg(long, value_parser = parse_socket_addr)]
        /// Address of llama.cpp instance that the balancer will forward requests to. If not
        /// provided, then `--local-llamacpp-addr` will be used
        external_llamacpp_addr: Option<SocketAddr>,

        #[arg(long, value_parser = parse_socket_addr)]
        /// Address of the local llama.cpp instance that the agent will monitor
        local_llamacpp_addr: SocketAddr,

        #[arg(long)]
        /// API key for the llama.cpp instance (optional)
        llamacpp_api_key: Option<String>,

        #[arg(long, value_parser = parse_socket_addr)]
        /// Address of the management server that the agent will report to
        management_addr: SocketAddr,

        #[arg(long, default_value = "10000", value_parser = parse_duration)]
        /// Interval (in milliseconds) at which the agent will report the status of the llama.cpp instance
        monitoring_interval: Duration,

        #[arg(long)]
        /// Name of the agent (optional)
        name: Option<String>,

        #[arg(long)]
        /// Flag whether to check the model served by llama.cpp and reject requests for other models
        check_model: bool,
    },
    /// Balances incoming requests to llama.cpp instances and optionally provides a web dashboard
    Balancer {
        #[arg(long, default_value = "10000", value_parser = parse_duration)]
        /// The request timeout (in milliseconds). For all requests that a timely response from an
        /// upstream isn't received for, the 504 (Gateway Timeout) error is issued.
        buffered_request_timeout: Duration,

        #[arg(long, value_parser = parse_socket_addr)]
        /// Address of the management server that the balancer will report to
        management_addr: SocketAddr,

        #[arg(
            long = "management-cors-allowed-host",
            help = "Allowed CORS host (can be specified multiple times)",
            action = clap::ArgAction::Append
        )]
        management_cors_allowed_hosts: Vec<String>,

        #[cfg(feature = "web_dashboard")]
        #[arg(long)]
        /// Enable the web management dashboard
        management_dashboard_enable: bool,

        #[arg(long, default_value = "30")]
        /// The maximum number of buffered requests. Like with usual requests, the request timeout
        /// is also applied to buffered ones. If the maximum number is reached, all new requests are
        /// rejected with the 429 (Too Many Requests) error.
        max_buffered_requests: usize,

        #[arg(long, value_parser = parse_socket_addr)]
        /// Address of the reverse proxy server
        reverseproxy_addr: SocketAddr,

        #[arg(long)]
        /// Rewrite the host header of incoming requests so that it matches the upstream server
        /// instead of the reverse client server
        rewrite_host_header: bool,

        #[arg(long)]
        /// Enable the web metrics endpoint
        metrics_endpoint_enable: bool,

        #[arg(long)]
        /// Enable the slots endpoint (not recommended)
        slots_endpoint_enable: bool,

        #[arg(long)]
        /// Flag to check the model served by llama.cpp and reject requests for other models
        check_model: bool,

        #[cfg(feature = "statsd_reporter")]
        #[arg(long, value_parser = parse_socket_addr)]
        /// Address of the statsd server to report metrics to
        statsd_addr: Option<SocketAddr>,

        #[cfg(feature = "statsd_reporter")]
        #[arg(long, default_value = "paddler")]
        /// Prefix for statsd metrics
        statsd_prefix: String,

        #[cfg(feature = "statsd_reporter")]
        #[arg(long, default_value = "10000", value_parser = parse_duration)]
        /// Interval (in milliseconds) at which the balancer will report metrics to statsd
        statsd_reporting_interval: Duration,
    },
    #[cfg(feature = "ratatui_dashboard")]
    /// Command-line dashboard for monitoring the balancer
    Dashboard {
        #[arg(long, value_parser = parse_socket_addr)]
        /// Address of the management server that the dashboard will connect to
        management_addr: SocketAddr,
    },
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Agent {
            external_llamacpp_addr,
            local_llamacpp_addr,
            llamacpp_api_key,
            management_addr,
            monitoring_interval,
            name,
            check_model,
        }) => cmd::agent::handle(
            match external_llamacpp_addr {
                Some(addr) => addr.to_owned(),
                None => local_llamacpp_addr.to_owned(),
            },
            local_llamacpp_addr.to_owned(),
            llamacpp_api_key.to_owned(),
            management_addr.to_owned(),
            monitoring_interval.to_owned(),
            name.to_owned(),
            *check_model
        ),
        Some(Commands::Balancer {
            buffered_request_timeout,
            management_addr,
            management_cors_allowed_hosts,
            #[cfg(feature = "web_dashboard")]
            management_dashboard_enable,
            max_buffered_requests,
            metrics_endpoint_enable,
            reverseproxy_addr,
            rewrite_host_header,
            check_model,
            slots_endpoint_enable,
            #[cfg(feature = "statsd_reporter")]
            statsd_addr,
            #[cfg(feature = "statsd_reporter")]
            statsd_prefix,
            #[cfg(feature = "statsd_reporter")]
            statsd_reporting_interval,
        }) => {
            #[cfg(feature = "web_dashboard")]
            initialize_instance(ESBUILD_META_CONTENTS);

            cmd::balancer::handle(
                *buffered_request_timeout,
                management_addr,
                management_cors_allowed_hosts.to_owned(),
                #[cfg(feature = "web_dashboard")]
                management_dashboard_enable.to_owned(),
                *max_buffered_requests,
                metrics_endpoint_enable.to_owned(),
                reverseproxy_addr,
                rewrite_host_header.to_owned(),
                *check_model,
                slots_endpoint_enable.to_owned(),
                #[cfg(feature = "statsd_reporter")]
                statsd_addr.to_owned(),
                #[cfg(feature = "statsd_reporter")]
                statsd_prefix.to_owned(),
                #[cfg(feature = "statsd_reporter")]
                statsd_reporting_interval.to_owned(),
            )
        }
        #[cfg(feature = "ratatui_dashboard")]
        Some(Commands::Dashboard {
            management_addr,
        }) => cmd::dashboard::handle(management_addr),
        None => Ok(()),
    }
}
