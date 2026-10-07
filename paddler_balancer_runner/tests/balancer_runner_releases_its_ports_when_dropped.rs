use std::net::TcpListener;

use tokio_util::sync::CancellationToken;

use paddler_balancer_runner::balancer_runner::BalancerRunner;

use crate::ephemeral_balancer_runner_params::ephemeral_balancer_runner_params;

#[tokio::test]
async fn balancer_runner_releases_its_ports_when_dropped() {
    let runner = BalancerRunner::start(ephemeral_balancer_runner_params(CancellationToken::new()))
        .await
        .expect("a runner on ephemeral ports must start");
    let addresses = runner.addresses;

    drop(runner);

    TcpListener::bind(addresses.management)
        .expect("the management port must be released after the runner is dropped");
    TcpListener::bind(addresses.inference)
        .expect("the inference port must be released after the runner is dropped");
}
