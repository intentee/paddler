pub mod configuration;
pub mod statsd_udp_sink;

use std::net::UdpSocket;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use cadence::StatsdClient;
use log::error;
use tokio::select;
use tokio::time::MissedTickBehavior;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use trzcina::Service;

use crate::agent_controller_pool::AgentControllerPool;
use crate::balancer_metrics::BalancerMetrics;
use crate::buffered_request_manager::BufferedRequestManager;
use crate::statsd_service::configuration::Configuration as StatsdServiceConfiguration;
use crate::statsd_service::statsd_udp_sink::StatsdUdpSink;

pub struct StatsdService {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub buffered_request_manager: Arc<BufferedRequestManager>,
    pub configuration: StatsdServiceConfiguration,
    pub statsd_prefix: String,
}

impl StatsdService {
    async fn report_metrics_until_shutdown(
        self: Box<Self>,
        shutdown: CancellationToken,
    ) -> Result<()> {
        let statsd_sink = StatsdUdpSink {
            socket: UdpSocket::bind("0.0.0.0:0")?,
            statsd_addr: self.configuration.statsd_addr.socket_addr,
        };
        let client = StatsdClient::builder(&self.statsd_prefix, statsd_sink).build();

        let mut ticker = interval(self.configuration.statsd_reporting_interval);

        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            select! {
                () = shutdown.cancelled() => break Ok(()),
                _ = ticker.tick() => {
                    if let Err(err) = BalancerMetrics::gather(
                        &self.agent_controller_pool,
                        &self.buffered_request_manager,
                    )
                    .report_to(&client)
                    {
                        error!("Failed to report metrics: {err}");
                    }
                }
            }
        }
    }
}

#[async_trait]
impl Service for StatsdService {
    fn name(&self) -> &'static str {
        "balancer::statsd_service"
    }

    async fn run(self: Box<Self>, shutdown: CancellationToken) -> Result<()> {
        self.report_metrics_until_shutdown(shutdown).await
    }
}
