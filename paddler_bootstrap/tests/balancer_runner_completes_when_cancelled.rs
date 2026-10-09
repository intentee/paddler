use tokio_util::sync::CancellationToken;

use paddler_bootstrap::balancer_runner::BalancerRunner;

use crate::ephemeral_balancer_runner_params::ephemeral_balancer_runner_params;

#[tokio::test]
async fn balancer_runner_completes_when_cancelled() {
    let mut runner =
        BalancerRunner::start(ephemeral_balancer_runner_params(CancellationToken::new()))
            .await
            .expect("a runner on ephemeral ports must start");

    runner.cancel();

    runner
        .wait_for_completion()
        .await
        .expect("a cancelled runner must complete cleanly");
}
