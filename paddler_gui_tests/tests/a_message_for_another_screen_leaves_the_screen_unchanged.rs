use paddler_gui::message::Message;
use paddler_gui_tests::app_driver::AppDriver;

#[tokio::test(flavor = "multi_thread")]
async fn a_message_for_another_screen_leaves_the_screen_unchanged() {
    let mut app = AppDriver::open().await.expect("the app must open");

    let _ignored_task = app.update(Message::BalancerStopped);

    app.find("Join a cluster")
        .expect("a message for another screen must leave the home screen in place");
}
