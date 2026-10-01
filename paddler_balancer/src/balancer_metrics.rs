use cadence::Gauged as _;
use cadence::MetricError;
use cadence::StatsdClient;
use indoc::formatdoc;

use crate::agent_controller_pool::AgentControllerPool;
use crate::agent_controller_pool_total_slots::AgentControllerPoolTotalSlots;
use crate::balancer_gauge::BalancerGauge;
use crate::buffered_request_manager::BufferedRequestManager;

pub struct BalancerMetrics {
    pub requests_buffered: u64,
    pub slots_processing: u64,
    pub slots_total: u64,
}

impl BalancerMetrics {
    #[must_use]
    pub fn gather(
        agent_controller_pool: &AgentControllerPool,
        buffered_request_manager: &BufferedRequestManager,
    ) -> Self {
        let AgentControllerPoolTotalSlots {
            slots_processing,
            slots_total,
        } = agent_controller_pool.total_slots();

        Self {
            requests_buffered: buffered_request_manager.buffered_request_counter.get(),
            slots_processing,
            slots_total,
        }
    }

    const fn gauges(&self) -> [BalancerGauge; 3] {
        [
            BalancerGauge {
                help: "Number of processing slots",
                name: "slots_processing",
                value: self.slots_processing,
            },
            BalancerGauge {
                help: "Number of total slots",
                name: "slots_total",
                value: self.slots_total,
            },
            BalancerGauge {
                help: "Number of buffered requests",
                name: "requests_buffered",
                value: self.requests_buffered,
            },
        ]
    }

    pub fn report_to(&self, client: &StatsdClient) -> Result<(), MetricError> {
        for BalancerGauge { name, value, .. } in self.gauges() {
            client.gauge(name, value)?;
        }

        client.flush()
    }

    #[must_use]
    pub fn to_prometheus_text(&self, prefix: &str) -> String {
        self.gauges()
            .map(|BalancerGauge { help, name, value }| {
                formatdoc! {"
                    # HELP {prefix}{name} {help}
                    # TYPE {prefix}{name} gauge
                    {prefix}{name} {value}
                "}
            })
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering;

    use cadence::MetricSink;
    use cadence::StatsdClient;
    use indoc::indoc;

    use super::BalancerMetrics;

    struct SinkFailingOnCall {
        calls: AtomicUsize,
        failing_call: usize,
    }

    impl MetricSink for SinkFailingOnCall {
        fn emit(&self, metric: &str) -> io::Result<usize> {
            if self.calls.fetch_add(1, Ordering::SeqCst) == self.failing_call {
                return Err(io::Error::other("the statsd sink refused the metric"));
            }

            Ok(metric.len())
        }

        fn flush(&self) -> io::Result<()> {
            if self.calls.fetch_add(1, Ordering::SeqCst) == self.failing_call {
                return Err(io::Error::other("the statsd sink refused to flush"));
            }

            Ok(())
        }
    }

    const NO_FAILING_CALL: usize = usize::MAX;
    const FLUSH_CALL: usize = 3;

    const METRICS: BalancerMetrics = BalancerMetrics {
        requests_buffered: 3,
        slots_processing: 1,
        slots_total: 4,
    };

    fn report_with_sink_failing_on_call(failing_call: usize) -> bool {
        let client = StatsdClient::from_sink(
            "paddler",
            SinkFailingOnCall {
                calls: AtomicUsize::new(0),
                failing_call,
            },
        );

        METRICS.report_to(&client).is_err()
    }

    #[test]
    fn reports_every_gauge_and_flushes() {
        assert!(!report_with_sink_failing_on_call(NO_FAILING_CALL));
    }

    #[test]
    fn stops_at_the_first_gauge_the_sink_refuses() {
        assert!(report_with_sink_failing_on_call(0));
    }

    #[test]
    fn reports_a_flush_the_sink_refuses() {
        assert!(report_with_sink_failing_on_call(FLUSH_CALL));
    }

    #[test]
    fn renders_every_gauge_in_the_prometheus_text_format() {
        assert_eq!(
            METRICS.to_prometheus_text("paddler_"),
            indoc! {"
                # HELP paddler_slots_processing Number of processing slots
                # TYPE paddler_slots_processing gauge
                paddler_slots_processing 1

                # HELP paddler_slots_total Number of total slots
                # TYPE paddler_slots_total gauge
                paddler_slots_total 4

                # HELP paddler_requests_buffered Number of buffered requests
                # TYPE paddler_requests_buffered gauge
                paddler_requests_buffered 3
            "}
        );
    }
}
