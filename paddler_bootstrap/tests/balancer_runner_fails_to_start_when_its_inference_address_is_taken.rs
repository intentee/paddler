use std::net::TcpListener;

use paddler_bootstrap::balancer_runner::BalancerRunner;
use paddler_bootstrap::bootstrap_error::BootstrapError;
use tokio_util::sync::CancellationToken;

use crate::ephemeral_balancer_runner_params::ephemeral_balancer_runner_params;

#[tokio::test]
async fn balancer_runner_fails_to_start_when_its_inference_address_is_taken() {
    let occupying_listener =
        TcpListener::bind("127.0.0.1:0").expect("an ephemeral loopback port must be bindable");
    let taken_addr = occupying_listener
        .local_addr()
        .expect("a bound listener must report its address");

    let mut params = ephemeral_balancer_runner_params(CancellationToken::new());

    params.inference_service_configuration.addr = taken_addr;

    let start_error = BalancerRunner::start(params)
        .await
        .err()
        .expect("the runner must not start while its inference address is taken");

    assert!(matches!(
        start_error,
        BootstrapError::InferenceBindFailed { addr, .. } if addr == taken_addr
    ));
}
