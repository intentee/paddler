use tokio_util::sync::CancellationToken;

use paddler_bootstrap::agent_runner::AgentRunner;

use crate::agent_runner_params_for_unreachable_balancer::agent_runner_params_for_unreachable_balancer;

#[tokio::test]
async fn agent_runner_completes_when_its_parent_token_is_cancelled() {
    let parent_token = CancellationToken::new();
    let runner = AgentRunner::start(agent_runner_params_for_unreachable_balancer(
        parent_token.clone(),
    ));

    parent_token.cancel();

    runner
        .wait_for_completion()
        .await
        .expect("an agent runner must complete cleanly when its parent token is cancelled");
}
