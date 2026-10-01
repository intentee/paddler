use paddler_gui::home_message::HomeMessage;
use paddler_gui::message::Message;
use paddler_gui_tests::app_driver::AppDriver;

#[tokio::test(flavor = "multi_thread")]
async fn cancelling_the_join_form_returns_to_the_home_screen() {
    let mut app = AppDriver::open().await.expect("the app must open");
    let _form_task = app.update(Message::Home(HomeMessage::JoinBalancer));

    let _cancel_task = app
        .click("Cancel")
        .expect("the join form must offer to cancel");

    app.find("Start a cluster")
        .expect("cancelling the join form must return to the home screen");
}
