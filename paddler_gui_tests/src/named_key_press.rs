use iced_test::core::Event;
use iced_test::core::keyboard::Event as KeyboardEvent;
use iced_test::core::keyboard::Key;
use iced_test::core::keyboard::Location;
use iced_test::core::keyboard::Modifiers;
use iced_test::core::keyboard::key::Code;
use iced_test::core::keyboard::key::Named;
use iced_test::core::keyboard::key::Physical;

#[must_use]
pub const fn named_key_press(named_key: Named, key_code: Code, modifiers: Modifiers) -> Event {
    Event::Keyboard(KeyboardEvent::KeyPressed {
        key: Key::Named(named_key),
        modified_key: Key::Named(named_key),
        physical_key: Physical::Code(key_code),
        location: Location::Standard,
        modifiers,
        text: None,
        repeat: false,
    })
}
