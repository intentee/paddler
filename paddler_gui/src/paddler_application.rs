use iced::Program;
use iced::Size;
use iced::Theme;
use iced::application;
use iced::application::Application;

use crate::app::App;
use crate::message::Message;

#[must_use]
pub fn paddler_application()
-> Application<impl Program<State = App, Message = Message, Theme = Theme>> {
    application(App::new, App::update, App::view)
        .font(include_bytes!(
            "../../resources/fonts/JetBrainsMono-Regular.ttf"
        ))
        .font(include_bytes!(
            "../../resources/fonts/JetBrainsMono-Bold.ttf"
        ))
        .theme(Theme::Light)
        .window_size(Size::new(800.0, 800.0))
        .subscription(App::subscription)
}
