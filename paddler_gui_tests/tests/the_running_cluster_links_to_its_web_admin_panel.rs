#![cfg(feature = "web_admin_panel")]

use reqwest::get;

use paddler_gui::message::Message;
use paddler_gui::running_balancer_message::RunningBalancerMessage;
use paddler_gui_tests::app_driver::AppDriver;
use paddler_test_cluster_harness::ephemeral_loopback_addr::EPHEMERAL_LOOPBACK_ADDR;

#[tokio::test(flavor = "multi_thread")]
async fn the_running_cluster_links_to_its_web_admin_panel() {
    let mut app = AppDriver::open().await.expect("the app must open");
    let mut cluster = app
        .start_cluster(EPHEMERAL_LOOPBACK_ADDR.to_string())
        .await
        .expect("the cluster must start");
    let link_messages = app
        .messages_from_clicking("Open in browser")
        .expect("the running cluster must link to its web admin panel");
    let [Message::RunningBalancer(RunningBalancerMessage::OpenUrl(web_admin_panel_url))] =
        link_messages.as_slice()
    else {
        panic!("following the link must only open the web admin panel: {link_messages:?}");
    };

    get(web_admin_panel_url)
        .await
        .expect("the linked web admin panel must accept connections")
        .error_for_status()
        .expect("the linked web admin panel must render");

    let _stop_task = app
        .click("Stop cluster")
        .expect("the running cluster must offer to stop");
    app.deliver_until(&mut cluster.messages, |message| {
        matches!(message, Message::BalancerStopped)
    })
    .await
    .expect("the stopped cluster must report its stop");
}
