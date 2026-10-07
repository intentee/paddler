use tokio_util::sync::CancellationToken;
use trzcina::ServiceShutdownOptions;

use paddler_agent_runner::agent_runner::AgentRunner;
use paddler_agent_runner::agent_runner_config::AgentRunnerConfig;
use paddler_agent_runner::agent_runner_params::AgentRunnerParams;
use paddler_balancer_runner::balancer_runner::BalancerRunner;
use paddler_messaging::balancer_connection::BalancerConnection;

use crate::ephemeral_balancer_runner_params::ephemeral_balancer_runner_params;

#[tokio::test]
async fn agent_runner_reports_its_connection_to_the_balancer() {
    let balancer_runner =
        BalancerRunner::start(ephemeral_balancer_runner_params(CancellationToken::new()))
            .await
            .expect("a runner on ephemeral ports must start");
    let agent_runner = AgentRunner::start(AgentRunnerParams {
        runner_config: AgentRunnerConfig {
            agent_name: Some("test-agent".to_owned()),
            management_address: balancer_runner.addresses.management.to_string(),
            slots: 1,
        },
        cancellation_token: CancellationToken::new(),
        shutdown_options: ServiceShutdownOptions::default(),
    });
    let mut balancer_connection_rx = agent_runner.balancer_connection_rx.clone();

    balancer_connection_rx
        .wait_for(|balancer_connection| *balancer_connection == BalancerConnection::Connected)
        .await
        .expect("the agent must report its connection to the balancer");

    agent_runner.cancel();
    agent_runner
        .wait_for_completion()
        .await
        .expect("a cancelled agent runner must complete cleanly");
    balancer_runner.cancel();
    balancer_runner
        .wait_for_completion()
        .await
        .expect("a cancelled balancer runner must complete cleanly");
}
