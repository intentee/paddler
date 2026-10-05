use tokio_util::sync::CancellationToken;

use paddler_bootstrap::balancer_runner::BalancerRunner;

use crate::ephemeral_balancer_runner_params::ephemeral_balancer_runner_params;

#[tokio::test]
async fn balancer_runner_completes_when_its_parent_token_is_cancelled() {
    let parent_token = CancellationToken::new();
    let runner = BalancerRunner::start(ephemeral_balancer_runner_params(parent_token.clone()))
        .await
        .expect("a runner on ephemeral ports must start");

    parent_token.cancel();

    runner
        .wait_for_completion()
        .await
        .expect("a runner must complete cleanly when its parent token is cancelled");
}
