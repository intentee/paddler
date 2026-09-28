use paddler_bootstrap::agent_runner::AgentRunner;
use paddler_bootstrap::agent_runner_params::AgentRunnerParams;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn agent_runner_completes_after_cancel_when_management_server_is_unresponsive() {
    let unresponsive_management_server = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("the unresponsive management server must bind an ephemeral port");
    let management_addr = unresponsive_management_server
        .local_addr()
        .expect("a bound listener must report its address");

    let mut runner = AgentRunner::start(AgentRunnerParams {
        agent_name: Some("test-agent".to_owned()),
        cancellation_token: CancellationToken::new(),
        management_address: management_addr.to_string(),
        slots: 1,
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
