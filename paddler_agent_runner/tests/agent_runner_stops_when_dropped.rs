use tokio_util::sync::CancellationToken;

use paddler_agent_runner::agent_runner::AgentRunner;

use crate::agent_runner_params_for_unreachable_balancer::agent_runner_params_for_unreachable_balancer;

#[tokio::test]
async fn agent_runner_stops_when_dropped() {
    let runner = AgentRunner::start(agent_runner_params_for_unreachable_balancer(
        CancellationToken::new(),
    ));

    drop(runner);
}
