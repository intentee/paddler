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

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::net::SocketAddr;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::AtomicI32;
    use std::sync::atomic::AtomicU64;
    use std::time::Duration;

    use cadence::BufferedSpyMetricSink;
    use cadence::ErrorKind;
    use cadence::MetricError;
    use cadence::SpyMetricSink;
    use parking_lot::RwLock;
    use tokio::net::UdpSocket as TokioUdpSocket;
    use tokio::sync::mpsc;

    use super::*;
    use crate::agent_controller::AgentController;
    use crate::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
    use crate::embedding_sender_collection::EmbeddingSenderCollection;
    use crate::generate_tokens_sender_collection::GenerateTokensSenderCollection;
    use crate::model_metadata_sender_collection::ModelMetadataSenderCollection;
    use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
    use paddler_messaging::atomic_value::AtomicValue;

    const REPORTING_INTERVAL: Duration = Duration::from_secs(1);
    const STATSD_PREFIX: &str = "paddler";

    fn register_agent_controller_with_slots(
        pool: &AgentControllerPool,
        agent_id: &str,
        slots_processing: i32,
        slots_total: i32,
    ) {
        let (agent_message_tx, _agent_message_rx) = mpsc::unbounded_channel();

        pool.register_agent_controller(
            agent_id.to_owned(),
            Arc::new(AgentController {
                agent_message_tx,
                chat_template_override_sender_collection: Arc::new(
                    ChatTemplateOverrideSenderCollection::default(),
                ),
                connection_close: CancellationToken::new(),
                desired_slots_total: AtomicValue::<AtomicI32>::new(0),
                download_current: AtomicValue::<AtomicU64>::new(0),
                download_filename: RwLock::new(None),
                download_indeterminate: AtomicValue::<AtomicBool>::new(true),
                download_total: AtomicValue::<AtomicU64>::new(0),
                embedding_sender_collection: Arc::new(EmbeddingSenderCollection::default()),
                generate_tokens_sender_collection: Arc::new(
                    GenerateTokensSenderCollection::default(),
                ),
                id: agent_id.to_owned(),
                issues: RwLock::new(BTreeSet::new()),
                model_metadata_sender_collection: Arc::new(
                    ModelMetadataSenderCollection::default(),
                ),
                model_path: RwLock::new(None),
                name: None,
                newest_update_version: AtomicValue::<AtomicI32>::new(0),
                slots_processing: AtomicValue::<AtomicI32>::new(slots_processing),
                slots_total: AtomicValue::<AtomicI32>::new(slots_total),
                state_application_status_code: AtomicValue::<AtomicI32>::new(
                    AgentStateApplicationStatus::Fresh as i32,
                ),
                uses_chat_template_override: AtomicValue::<AtomicBool>::new(false),
            }),
        )
        .unwrap();
    }

    fn build_service(statsd_addr: SocketAddr) -> StatsdService {
        let agent_controller_pool = Arc::new(AgentControllerPool::default());
        let buffered_request_manager = Arc::new(BufferedRequestManager::new(
            agent_controller_pool.clone(),
            REPORTING_INTERVAL,
            10,
        ));

        StatsdService {
            agent_controller_pool,
            buffered_request_manager,
            configuration: StatsdServiceConfiguration {
                statsd_addr,
                statsd_prefix: STATSD_PREFIX.to_owned(),
                statsd_reporting_interval: REPORTING_INTERVAL,
            },
        }
    }

    #[test]
    fn name_identifies_the_statsd_service() {
        let service = build_service(SocketAddr::from(([127, 0, 0, 1], 0)));

        assert_eq!(service.name(), "balancer::statsd_service");
    }

    #[tokio::test]
    async fn report_metrics_emits_a_gauge_datagram_for_each_pool_metric() {
        let receiver = TokioUdpSocket::bind("127.0.0.1:0").await.unwrap();
        let receiver_addr = receiver.local_addr().unwrap();
        let service = build_service(receiver_addr);

        let sender_socket = UdpSocket::bind("0.0.0.0:0").unwrap();
        let sink = UdpMetricSink::from(receiver_addr, sender_socket).unwrap();
        let client = StatsdClient::builder(STATSD_PREFIX, sink).build();

        service.report_metrics(&client).unwrap();

        let mut received_lines: Vec<String> = Vec::new();
        let mut datagram = [0_u8; 1024];

        for _ in 0..3 {
            let byte_count = receiver.recv(&mut datagram).await.unwrap();

            received_lines.push(String::from_utf8(datagram[..byte_count].to_vec()).unwrap());
        }

        assert!(received_lines.contains(&"paddler.slots_processing:0|g".to_owned()));
        assert!(received_lines.contains(&"paddler.slots_total:0|g".to_owned()));
        assert!(received_lines.contains(&"paddler.requests_buffered:0|g".to_owned()));
    }

    #[tokio::test]
    async fn run_reports_on_first_tick_then_stops_on_cancellation() {
        let receiver = TokioUdpSocket::bind("127.0.0.1:0").await.unwrap();
        let receiver_addr = receiver.local_addr().unwrap();
        let service = Box::new(build_service(receiver_addr));

        let shutdown = CancellationToken::new();
        let run_handle = tokio::spawn(service.run(shutdown.clone()));

        let mut datagram = [0_u8; 1024];
        let byte_count = receiver.recv(&mut datagram).await.unwrap();
        let first_line = String::from_utf8(datagram[..byte_count].to_vec()).unwrap();

        let expected_first_tick_lines = [
            "paddler.slots_processing:0|g".to_owned(),
            "paddler.slots_total:0|g".to_owned(),
            "paddler.requests_buffered:0|g".to_owned(),
        ];

        assert!(expected_first_tick_lines.contains(&first_line));

        shutdown.cancel();

        assert!(run_handle.await.unwrap().is_ok());
    }

    #[test]
    fn report_metrics_propagates_error_from_the_first_gauge_emit() {
        let (receiver, sink) = SpyMetricSink::new();

        drop(receiver);

        let client = StatsdClient::builder(STATSD_PREFIX, sink).build();
        let service = build_service(SocketAddr::from(([127, 0, 0, 1], 0)));

        let result = service.report_metrics(&client);

        assert!(result.err().unwrap().is::<MetricError>());
    }

    #[test]
    fn report_metrics_propagates_error_from_the_second_gauge_emit() {
        let (receiver, sink) = SpyMetricSink::with_capacity(1);
        let client = StatsdClient::builder(STATSD_PREFIX, sink).build();
        let service = build_service(SocketAddr::from(([127, 0, 0, 1], 0)));

        let result = service.report_metrics(&client);

        assert!(result.err().unwrap().is::<MetricError>());
        assert_eq!(receiver.len(), 1);
    }

    #[test]
    fn report_metrics_propagates_error_from_the_third_gauge_emit() {
        let (receiver, sink) = SpyMetricSink::with_capacity(2);
        let client = StatsdClient::builder(STATSD_PREFIX, sink).build();
        let service = build_service(SocketAddr::from(([127, 0, 0, 1], 0)));

        let result = service.report_metrics(&client);

        assert!(result.err().unwrap().is::<MetricError>());
        assert_eq!(receiver.len(), 2);
    }

    #[test]
    fn report_metrics_propagates_error_from_the_flush() {
        let (receiver, sink) = BufferedSpyMetricSink::new();
        let client = StatsdClient::builder(STATSD_PREFIX, sink).build();
        let service = build_service(SocketAddr::from(([127, 0, 0, 1], 0)));

        drop(receiver);

        let result = service.report_metrics(&client);

        assert!(result.err().unwrap().is::<MetricError>());
    }

    #[test]
    fn report_metrics_rejects_negative_slots_processing() {
        let service = build_service(SocketAddr::from(([127, 0, 0, 1], 0)));

        register_agent_controller_with_slots(&service.agent_controller_pool, "agent", -1, 0);

        let client = StatsdClient::builder(STATSD_PREFIX, SpyMetricSink::new().1).build();
        let result = service.report_metrics(&client);

        assert_eq!(
            result.err().unwrap().to_string(),
            "slots_processing count is negative"
        );
    }

    #[test]
    fn report_metrics_rejects_negative_slots_total() {
        let service = build_service(SocketAddr::from(([127, 0, 0, 1], 0)));

        register_agent_controller_with_slots(&service.agent_controller_pool, "agent", 0, -1);

        let client = StatsdClient::builder(STATSD_PREFIX, SpyMetricSink::new().1).build();
        let result = service.report_metrics(&client);

        assert_eq!(
            result.err().unwrap().to_string(),
            "slots_total count is negative"
        );
    }

    #[test]
    fn report_metrics_rejects_negative_requests_buffered() {
        let service = build_service(SocketAddr::from(([127, 0, 0, 1], 0)));

        service
            .buffered_request_manager
            .buffered_request_counter
            .decrement();

        let client = StatsdClient::builder(STATSD_PREFIX, SpyMetricSink::new().1).build();
        let result = service.report_metrics(&client);

        assert_eq!(
            result.err().unwrap().to_string(),
            "requests_buffered count is negative"
        );
    }

    #[test]
    fn log_statsd_error_logs_the_metric_error() {
        log::set_max_level(log::LevelFilter::Trace);

        log_statsd_error(MetricError::from((
            ErrorKind::InvalidInput,
            "statsd error fixture",
        )));
    }
}
