use std::collections::BTreeSet;
use std::time::Duration;

use paddler_balancer::statsd_service::configuration::Configuration as StatsdServiceConfiguration;
use paddler_bootstrap::balancer_runner::BalancerRunner;
use tokio::net::UdpSocket;
use tokio_util::sync::CancellationToken;

use crate::ephemeral_balancer_runner_params::ephemeral_balancer_runner_params;

const REPORTING_INTERVAL_LONGER_THAN_THE_TEST: Duration = Duration::from_hours(1);
const MAX_STATSD_DATAGRAM_BYTES: usize = 1024;

#[tokio::test]
async fn balancer_reports_its_gauges_to_statsd() {
    let statsd_socket = UdpSocket::bind("127.0.0.1:0")
        .await
        .expect("the statsd socket must bind");
    let mut params = ephemeral_balancer_runner_params(CancellationToken::new());

    params.statsd_service_configuration = Some(StatsdServiceConfiguration {
        statsd_addr: statsd_socket
            .local_addr()
            .expect("the statsd socket must report its address"),
        statsd_prefix: "paddler".to_owned(),
        statsd_reporting_interval: REPORTING_INTERVAL_LONGER_THAN_THE_TEST,
    });

    let mut runner = BalancerRunner::start(params)
        .await
        .expect("a runner reporting to statsd must start");
    let mut received_gauges = BTreeSet::new();
    let mut datagram = [0_u8; MAX_STATSD_DATAGRAM_BYTES];

    while received_gauges.len() < 3 {
        let datagram_length = statsd_socket
            .recv(&mut datagram)
            .await
            .expect("the balancer must send its gauges");

        received_gauges.insert(
            String::from_utf8(datagram[..datagram_length].to_vec())
                .expect("a statsd datagram must be text"),
        );
    }

    runner.cancel();
    runner
        .wait_for_completion()
        .await
        .expect("the runner must shut down cleanly");

    assert_eq!(
        received_gauges,
        BTreeSet::from([
            "paddler.requests_buffered:0|g".to_owned(),
            "paddler.slots_processing:0|g".to_owned(),
            "paddler.slots_total:0|g".to_owned(),
        ])
    );
}
