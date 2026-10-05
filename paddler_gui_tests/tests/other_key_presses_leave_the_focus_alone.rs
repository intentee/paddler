use iced_test::core::keyboard::Modifiers;
use iced_test::core::keyboard::key::Code;
use iced_test::core::keyboard::key::Named;

use paddler_gui::app::App;
use paddler_gui::message::Message;
use paddler_gui_tests::app_subscriptions::AppSubscriptions;
use paddler_gui_tests::named_key_press::named_key_press;

#[test]
fn other_key_presses_leave_the_focus_alone() {
    let (app, _boot_task) = App::new();
    let mut subscriptions =
        AppSubscriptions::of(&app).expect("the app must subscribe to its events");

    subscriptions.broadcast_unhandled(named_key_press(
        Named::Escape,
        Code::Escape,
        Modifiers::empty(),
    ));
    subscriptions.broadcast_unhandled(named_key_press(Named::Tab, Code::Tab, Modifiers::SHIFT));

    assert!(matches!(
        subscriptions
            .next_message()
            .expect("the tab press after the other key must produce a message"),
        Message::TabPressed { shift: true }
    ));
}
