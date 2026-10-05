use paddler_gui::message::Message;
use paddler_gui_tests::app_driver::AppDriver;

#[tokio::test(flavor = "multi_thread")]
async fn cancelling_a_starting_cluster_stops_it() {
    let mut app = AppDriver::open().await.expect("the app must open");
    let mut cluster = app
        .confirm_cluster_start(String::new())
        .expect("confirming the form must start the cluster");

    app.find("Starting...")
        .expect("the start form must show that the cluster is starting");

    let _cancel_task = app
        .click("Cancel")
        .expect("the starting cluster must offer to cancel");

    app.deliver_until(&mut cluster, |message| {
        matches!(message, Message::BalancerStopped)
    })
    .await
    .expect("cancelling a starting cluster must stop it");
    app.find("Join a cluster")
        .expect("a cancelled cluster must leave the app on the home screen");
}
