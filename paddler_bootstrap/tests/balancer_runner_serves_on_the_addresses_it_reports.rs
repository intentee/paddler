use paddler_bootstrap::balancer_runner::BalancerRunner;
use tokio_util::sync::CancellationToken;

use crate::ephemeral_balancer_runner_params::ephemeral_balancer_runner_params;

#[tokio::test]
async fn balancer_runner_serves_on_the_addresses_it_reports() {
    let runner = BalancerRunner::start(ephemeral_balancer_runner_params(CancellationToken::new()))
        .await
        .expect("a runner on ephemeral ports must start");

    let inference_health = reqwest::get(format!("http://{}/health", runner.addresses.inference))
        .await
        .expect("the inference service must accept connections on its reported address")
        .text()
        .await
        .expect("the inference health response must have a body");
    let management_health = reqwest::get(format!("http://{}/health", runner.addresses.management))
        .await
        .expect("the management service must accept connections on its reported address")
        .text()
        .await
        .expect("the management health response must have a body");

    assert_eq!(inference_health, "OK");
    assert_eq!(management_health, "OK");
}
