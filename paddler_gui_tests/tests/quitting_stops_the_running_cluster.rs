use paddler_gui::message::Message;
use paddler_gui_tests::app_driver::AppDriver;

#[tokio::test(flavor = "multi_thread")]
async fn quitting_stops_the_running_cluster() {
    let mut app = AppDriver::open().await.expect("the app must open");
    let mut cluster = app
        .start_cluster(String::new())
        .await
        .expect("the cluster must start");

    let _exit_task = app.update(Message::Quit);

    app.deliver_until(&mut cluster.messages, |message| {
        matches!(message, Message::BalancerStopped)
    })
    .await
    .expect("quitting must stop the running cluster");
}
