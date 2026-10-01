use iced_test::core::Event;
use iced_test::core::window::Event as WindowEvent;

use paddler_gui::app::App;
use paddler_gui::message::Message;
use paddler_gui_tests::app_subscriptions::AppSubscriptions;

#[test]
fn closing_the_window_quits_the_app() {
    let (app, _boot_task) = App::new();
    let mut subscriptions =
        AppSubscriptions::of(&app).expect("the app must subscribe to its events");

    subscriptions.broadcast_unhandled(Event::Window(WindowEvent::CloseRequested));

    assert!(matches!(
        subscriptions
            .next_message()
            .expect("closing the window must produce a message"),
        Message::Quit
    ));
}
