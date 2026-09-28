pub mod configuration;

use std::net::UdpSocket;
use std::sync::Arc;

use anyhow::Context as _;
use anyhow::Result;
use async_trait::async_trait;
use cadence::Gauged;
use cadence::MetricError;
use cadence::StatsdClient;
use cadence::UdpMetricSink;
use log::error;
use tokio::time::MissedTickBehavior;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use trzcina::Service;

use crate::agent_controller_pool::AgentControllerPool;
use crate::agent_controller_pool_total_slots::AgentControllerPoolTotalSlots;
use crate::buffered_request_manager::BufferedRequestManager;
use crate::statsd_service::configuration::Configuration as StatsdServiceConfiguration;

fn log_statsd_error(error: MetricError) {
    error!("Statsd error: {error}");
}

pub struct StatsdService {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub buffered_request_manager: Arc<BufferedRequestManager>,
    pub configuration: StatsdServiceConfiguration,
}

impl StatsdService {
    fn report_metrics(&self, client: &StatsdClient) -> Result<()> {
        let AgentControllerPoolTotalSlots {
            slots_processing,
            slots_total,
        } = self.agent_controller_pool.total_slots();
        let requests_buffered = self.buffered_request_manager.buffered_request_counter.get();

        let slots_processing =
            u64::try_from(slots_processing).context("slots_processing count is negative")?;
        let slots_total = u64::try_from(slots_total).context("slots_total count is negative")?;
        let requests_buffered =
            u64::try_from(requests_buffered).context("requests_buffered count is negative")?;

        client.gauge("slots_processing", slots_processing)?;
        client.gauge("slots_total", slots_total)?;
        client.gauge("requests_buffered", requests_buffered)?;
        client.flush()?;

        Ok(())
    }
}

#[async_trait]
impl Service for StatsdService {
    fn name(&self) -> &'static str {
        "balancer::statsd_service"
    }

    async fn run(self: Box<Self>, shutdown: CancellationToken) -> Result<()> {
        let statsd_sink_socket = UdpSocket::bind("0.0.0.0:0")?;
        let statsd_sink = UdpMetricSink::from(self.configuration.statsd_addr, statsd_sink_socket)?;

        let client = StatsdClient::builder(&self.configuration.statsd_prefix.clone(), statsd_sink)
            .with_error_handler(log_statsd_error)
            .build();

        let mut ticker = interval(self.configuration.statsd_reporting_interval);

        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                () = shutdown.cancelled() => break Ok(()),
                _ = ticker.tick() => {
                    if let Err(err) = self.report_metrics(&client) {
                        error!("Failed to report metrics: {err}");
                    }
                }
            }
        }
    }
}
