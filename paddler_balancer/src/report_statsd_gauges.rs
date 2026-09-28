use cadence::Gauged as _;
use cadence::MetricError;
use cadence::StatsdClient;

use crate::statsd_gauges::StatsdGauges;

pub fn report_statsd_gauges(
    client: &StatsdClient,
    StatsdGauges {
        requests_buffered,
        slots_processing,
        slots_total,
    }: &StatsdGauges,
) -> Result<(), MetricError> {
    client.gauge("slots_processing", *slots_processing)?;
    client.gauge("slots_total", *slots_total)?;
    client.gauge("requests_buffered", *requests_buffered)?;
    client.flush()
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering;

    use cadence::MetricSink;
    use cadence::StatsdClient;

    use super::report_statsd_gauges;
    use crate::statsd_gauges::StatsdGauges;

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

    fn report_with_sink_failing_on_call(failing_call: usize) -> bool {
        let client = StatsdClient::from_sink(
            "paddler",
            SinkFailingOnCall {
                calls: AtomicUsize::new(0),
                failing_call,
            },
        );

        report_statsd_gauges(
            &client,
            &StatsdGauges {
                requests_buffered: 3,
                slots_processing: 1,
                slots_total: 4,
            },
        )
        .is_err()
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
    fn stops_at_the_second_gauge_the_sink_refuses() {
        assert!(report_with_sink_failing_on_call(1));
    }

    #[test]
    fn stops_at_the_third_gauge_the_sink_refuses() {
        assert!(report_with_sink_failing_on_call(2));
    }

    #[test]
    fn reports_a_flush_the_sink_refuses() {
        assert!(report_with_sink_failing_on_call(3));
    }
}
