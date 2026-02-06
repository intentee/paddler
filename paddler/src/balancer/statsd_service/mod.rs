pub mod configuration;

use std::net::UdpSocket;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use cadence::Gauged;
use cadence::StatsdClient;
use cadence::UdpMetricSink;
use log::error;
use tokio::sync::broadcast;
use tokio::time::MissedTickBehavior;
use tokio::time::interval;

use crate::balancer::agent_controller_pool::AgentControllerPool;
use crate::balancer::agent_controller_pool_total_slots::AgentControllerPoolTotalSlots;
use crate::balancer::buffered_request_manager::BufferedRequestManager;
use crate::balancer::statsd_service::configuration::Configuration as StatsdServiceConfiguration;
use crate::service::Service;

pub struct StatsdService {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub buffered_request_manager: Arc<BufferedRequestManager>,
    pub configuration: StatsdServiceConfiguration,
}

impl StatsdService {
    async fn report_metrics(&self, client: &StatsdClient) -> Result<()> {
        let AgentControllerPoolTotalSlots {
            slots_processing,
            slots_total,
        } = self.agent_controller_pool.total_slots();
        let requests_buffered = self.buffered_request_manager.buffered_request_counter.get();

        client.gauge("slots_processing", slots_processing as u64)?;
        client.gauge("slots_total", slots_total as u64)?;
        client.gauge("requests_buffered", requests_buffered as u64)?;
        client.flush()?;

        Ok(())
    }
}

#[async_trait]
impl Service for StatsdService {
    fn name(&self) -> &'static str {
        "balancer::statsd_service"
    }

    async fn run(&mut self, mut shutdown: broadcast::Receiver<()>) -> Result<()> {
        let statsd_sink_socket = UdpSocket::bind("0.0.0.0:0")?;
        let statsd_sink = UdpMetricSink::from(self.configuration.statsd_addr, statsd_sink_socket)?;

        let client =
            StatsdClient::builder(&self.configuration.statsd_prefix.to_owned(), statsd_sink)
                .with_error_handler(|err| error!("Statsd error: {err}"))
                .build();

        let mut ticker = interval(self.configuration.statsd_reporting_interval);

        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = shutdown.recv() => break Ok(()),
                _ = ticker.tick() => {
                    if let Err(err) = self.report_metrics(&client).await {
                        error!("Failed to report metrics: {err}");
                    }
                }
            }
        }
    }
}
