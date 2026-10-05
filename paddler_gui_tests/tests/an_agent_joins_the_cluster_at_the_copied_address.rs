use paddler_gui::agent_running_message::AgentRunningMessage;
use paddler_gui::message::Message;
use paddler_gui_tests::app_driver::AppDriver;
use paddler_gui_tests::task_actions::TaskActions;
use paddler_messaging::balancer_connection::BalancerConnection;

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_joins_the_cluster_at_the_copied_address() {
    let mut cluster_app = AppDriver::open().await.expect("the cluster app must open");
    let mut cluster = cluster_app
        .start_cluster(String::new())
        .await
        .expect("the cluster must start");
    let copied_address = TaskActions::of(
        cluster_app
            .click("Copy address")
            .expect("the running cluster must offer to copy its address"),
    )
    .expect("copying the address must write to the clipboard")
    .next_clipboard_write()
    .await
    .expect("copying the address must write it to the clipboard");

    let mut agent_app = AppDriver::open().await.expect("the agent app must open");

    let mut agent = TaskActions::of(
        agent_app
            .join_cluster(copied_address.clone())
            .expect("the join form must offer to connect"),
    )
    .expect("connecting must run the agent");

    agent_app
        .deliver_until(&mut agent, |message| {
            matches!(
                message,
                Message::AgentRunning(AgentRunningMessage::BalancerConnectionChanged(
                    BalancerConnection::Connected
                ))
            )
        })
        .await
        .expect("the agent must connect to the copied address");
    agent_app
        .find(&format!("Connected to the cluster at {copied_address}"))
        .expect("the agent must show the cluster it joined");

    let _disconnect_task = agent_app
        .click("Disconnect")
        .expect("the running agent must offer to disconnect");
    agent_app
        .deliver_until(&mut agent, |message| {
            matches!(message, Message::AgentStopped)
        })
        .await
        .expect("the disconnected agent must stop");
    let _stop_task = cluster_app
        .click("Stop cluster")
        .expect("the running cluster must offer to stop");
    cluster_app
        .deliver_until(&mut cluster.messages, |message| {
            matches!(message, Message::BalancerStopped)
        })
        .await
        .expect("the stopped cluster must report its stop");
}
