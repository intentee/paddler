use paddler_gui::message::Message;
use paddler_gui_tests::app_driver::AppDriver;

#[tokio::test(flavor = "multi_thread")]
async fn stopping_a_running_cluster_returns_to_the_home_screen() {
    let mut app = AppDriver::open().await.expect("the app must open");
    let mut cluster = app
        .start_cluster(String::new())
        .await
        .expect("the cluster must start");

    let _stop_task = app
        .click("Stop cluster")
        .expect("the running cluster must offer to stop");
    app.find("Stopping...")
        .expect("a stopping cluster must say that it is stopping");
    app.deliver_until(&mut cluster.messages, |message| {
        matches!(message, Message::BalancerStopped)
    })
    .await
    .expect("the stopped cluster must report its stop");
    app.find("Join a cluster")
        .expect("a stopped cluster must return to the home screen");
}
