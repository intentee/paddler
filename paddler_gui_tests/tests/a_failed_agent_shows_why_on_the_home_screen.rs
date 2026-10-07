use std::sync::Arc;

use paddler_gui::message::Message;
use paddler_gui_tests::app_driver::AppDriver;
use paddler_service_thread::service_thread_error::ServiceThreadError;

#[tokio::test(flavor = "multi_thread")]
async fn a_failed_agent_shows_why_on_the_home_screen() {
    let mut app = AppDriver::open().await.expect("the app must open");
    let _agent_task = app
        .join_cluster("127.0.0.1:8060".to_owned())
        .expect("the join form must offer to connect");

    let _failed_task = app.update(Message::AgentFailed(Arc::new(
        ServiceThreadError::ServiceThreadPanicked,
    )));

    app.find("A Paddler service thread panicked")
        .expect("a failed agent must show why on the home screen");
}
