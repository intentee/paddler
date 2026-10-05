use std::net::TcpListener;

use paddler_gui::agent_running_message::AgentRunningMessage;
use paddler_gui::message::Message;
use paddler_gui_tests::app_driver::AppDriver;
use paddler_gui_tests::task_actions::TaskActions;
use paddler_messaging::balancer_connection::BalancerConnection;
use paddler_test_cluster_harness::ephemeral_loopback_addr::EPHEMERAL_LOOPBACK_ADDR;

#[tokio::test(flavor = "multi_thread")]
async fn quitting_stops_the_running_agent() {
    let unresponsive_balancer =
        TcpListener::bind(EPHEMERAL_LOOPBACK_ADDR).expect("the unresponsive balancer must bind");
    let mut app = AppDriver::open().await.expect("the app must open");
    let mut agent = TaskActions::of(
        app.join_cluster(
            unresponsive_balancer
                .local_addr()
                .expect("the unresponsive balancer must report its address")
                .to_string(),
        )
        .expect("the join form must offer to connect"),
    )
    .expect("connecting must run the agent");

    app.deliver_until(&mut agent, |message| {
        matches!(
            message,
            Message::AgentRunning(AgentRunningMessage::BalancerConnectionChanged(
                BalancerConnection::Connecting
            ))
        )
    })
    .await
    .expect("the agent must start connecting");

    let _exit_task = app.update(Message::Quit);

    app.deliver_until(&mut agent, |message| {
        matches!(message, Message::AgentStopped)
    })
    .await
    .expect("quitting must stop the running agent");
}
