use std::time::Duration;

use log::Level;
use tokio::task::yield_now;
use tokio_util::sync::CancellationToken;

use paddler_balancer::resolved_socket_addr::ResolvedSocketAddr;
use paddler_balancer::statsd_service::configuration::Configuration as StatsdServiceConfiguration;
use paddler_bootstrap::balancer_runner::BalancerRunner;
use paddler_bootstrap_tests::ephemeral_balancer_runner_params::ephemeral_balancer_runner_params;
use paddler_capturing_logger::captured_record::CapturedRecord;
use paddler_capturing_logger::capturing_logger::CapturingLogger;
use paddler_test_cluster_harness::constants::EPHEMERAL_LOOPBACK_ADDR;

const REPORTING_INTERVAL_LONGER_THAN_THE_TEST: Duration = Duration::from_hours(1);
const STATSD_SERVICE_TARGET: &str = "paddler_balancer::statsd_service";

#[tokio::test]
async fn balancer_logs_a_statsd_report_it_cannot_send() {
    CapturingLogger::install();

    let mut params = ephemeral_balancer_runner_params(CancellationToken::new());

    params.bootstrap_config.statsd_service_configuration = Some(StatsdServiceConfiguration {
        statsd_addr: ResolvedSocketAddr::from(EPHEMERAL_LOOPBACK_ADDR),
        statsd_reporting_interval: REPORTING_INTERVAL_LONGER_THAN_THE_TEST,
    });

    let mut runner = BalancerRunner::start(params)
        .await
        .expect("a runner reporting to statsd must start");

    while !CapturingLogger::records_for(STATSD_SERVICE_TARGET)
        .iter()
        .any(|CapturedRecord { level, .. }| *level == Level::Error)
    {
        yield_now().await;
    }

    runner.cancel();
    runner
        .wait_for_completion()
        .await
        .expect("a balancer whose statsd reports fail must keep running until it is cancelled");
}
