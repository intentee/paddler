use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use trzcina::ServiceShutdownOptions;

use paddler_agent_runner::agent_runner::AgentRunner;
use paddler_agent_runner::agent_runner_config::AgentRunnerConfig;
use paddler_agent_runner::agent_runner_params::AgentRunnerParams;
use paddler_test_cluster_harness::ephemeral_loopback_addr::EPHEMERAL_LOOPBACK_ADDR;

#[tokio::test]
async fn agent_runner_completes_after_cancel_when_management_server_is_unresponsive() {
    let unresponsive_management_server = TcpListener::bind(EPHEMERAL_LOOPBACK_ADDR)
        .await
        .expect("the unresponsive management server must bind an ephemeral port");
    let management_addr = unresponsive_management_server
        .local_addr()
        .expect("a bound listener must report its address");

    let runner = AgentRunner::start(AgentRunnerParams {
        runner_config: AgentRunnerConfig {
            agent_name: Some("test-agent".to_owned()),
            management_address: management_addr.to_string(),
            slots: 1,
        },
        cancellation_token: CancellationToken::new(),
        shutdown_options: ServiceShutdownOptions::default(),
    });

    let (_held_connection, _peer_addr) = unresponsive_management_server
        .accept()
        .await
        .expect("the agent must connect to the unresponsive management server");

    runner.cancel();
    runner
        .wait_for_completion()
        .await
        .expect("a cancelled agent runner must complete cleanly");
}
