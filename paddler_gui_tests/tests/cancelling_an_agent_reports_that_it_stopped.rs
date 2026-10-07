use std::net::TcpListener;

use futures::StreamExt as _;
use tokio_util::sync::CancellationToken;
use trzcina::ServiceShutdownOptions;

use paddler_agent_runner::agent_runner_config::AgentRunnerConfig;
use paddler_agent_runner::agent_runner_params::AgentRunnerParams;
use paddler_gui::agent_runner_messages::agent_runner_messages;
use paddler_gui::message::Message;
use paddler_test_cluster_harness::ephemeral_loopback_addr::EPHEMERAL_LOOPBACK_ADDR;

#[tokio::test(flavor = "multi_thread")]
async fn cancelling_an_agent_reports_that_it_stopped() {
    let unresponsive_balancer =
        TcpListener::bind(EPHEMERAL_LOOPBACK_ADDR).expect("the unresponsive balancer must bind");
    let cancellation_token = CancellationToken::new();
    let agent_messages = agent_runner_messages(AgentRunnerParams {
        runner_config: AgentRunnerConfig {
            agent_name: None,
            management_address: unresponsive_balancer
                .local_addr()
                .expect("the unresponsive balancer must report its address")
                .to_string(),
            slots: 1,
        },
        cancellation_token: cancellation_token.clone(),
        shutdown_options: ServiceShutdownOptions::default(),
    });

    cancellation_token.cancel();

    assert!(matches!(
        agent_messages.collect::<Vec<_>>().await.pop(),
        Some(Message::AgentStopped)
    ));
}
