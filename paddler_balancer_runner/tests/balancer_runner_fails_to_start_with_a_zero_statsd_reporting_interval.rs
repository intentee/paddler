use std::mem::discriminant;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use paddler_balancer::resolved_socket_addr::ResolvedSocketAddr;
use paddler_balancer::statsd_service::configuration::Configuration as StatsdServiceConfiguration;
use paddler_balancer_runner::balancer_runner::BalancerRunner;
use paddler_balancer_runner::balancer_runner_error::BalancerRunnerError;
use paddler_test_cluster_harness::ephemeral_loopback_addr::EPHEMERAL_LOOPBACK_ADDR;

use crate::ephemeral_balancer_runner_params::ephemeral_balancer_runner_params;

#[tokio::test]
async fn balancer_runner_fails_to_start_with_a_zero_statsd_reporting_interval() {
    let mut params = ephemeral_balancer_runner_params(CancellationToken::new());

    params.runner_config.statsd_service_configuration = Some(StatsdServiceConfiguration {
        statsd_addr: ResolvedSocketAddr::from(EPHEMERAL_LOOPBACK_ADDR),
        statsd_reporting_interval: Duration::ZERO,
    });

    let start_error = BalancerRunner::start(params)
        .await
        .err()
        .expect("the runner must not start with a zero statsd reporting interval");

    assert_eq!(
        discriminant(&start_error),
        discriminant(&BalancerRunnerError::StatsdReportingIntervalIsZero)
    );
}
