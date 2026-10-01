use iced_test::core::keyboard::Modifiers;
use iced_test::core::keyboard::key::Code;
use iced_test::core::keyboard::key::Named;

use paddler_gui::app::App;
use paddler_gui::message::Message;
use paddler_gui_tests::app_subscriptions::AppSubscriptions;
use paddler_gui_tests::named_key_press::named_key_press;

#[test]
fn pressing_shift_tab_asks_to_focus_the_previous_field() {
    let (app, _boot_task) = App::new();
    let mut subscriptions =
        AppSubscriptions::of(&app).expect("the app must subscribe to its events");

    subscriptions.broadcast_unhandled(named_key_press(Named::Tab, Code::Tab, Modifiers::SHIFT));

    assert!(matches!(
        subscriptions
            .next_message()
            .expect("pressing shift tab must produce a message"),
        Message::TabPressed { shift: true }
    ));
}
