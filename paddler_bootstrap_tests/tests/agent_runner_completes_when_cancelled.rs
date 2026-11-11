use tokio_util::sync::CancellationToken;

use paddler_bootstrap::agent_runner::AgentRunner;
use paddler_bootstrap_tests::agent_runner_params_for_unreachable_balancer::agent_runner_params_for_unreachable_balancer;

#[tokio::test]
async fn agent_runner_completes_when_cancelled() {
    let mut runner = AgentRunner::start(
        agent_runner_params_for_unreachable_balancer(CancellationToken::new())
            .expect("the model downloader must start"),
    );

    runner.cancel();

    runner
        .wait_for_completion()
        .await
        .expect("a cancelled agent runner must complete cleanly");
}
