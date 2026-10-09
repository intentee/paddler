use tokio_util::sync::CancellationToken;

use paddler_bootstrap::agent_runner::AgentRunner;

use crate::agent_runner_params_for_unreachable_balancer::agent_runner_params_for_unreachable_balancer;

#[tokio::test]
async fn agent_runner_completes_when_cancelled() {
    let mut runner = AgentRunner::start(agent_runner_params_for_unreachable_balancer(
        CancellationToken::new(),
    ));

    runner.cancel();

    runner
        .wait_for_completion()
        .await
        .expect("a cancelled agent runner must complete cleanly");
}
