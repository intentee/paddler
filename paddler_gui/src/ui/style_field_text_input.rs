use iced::Background;
use iced::Theme;
use iced::widget::text_input;

use super::variables::BORDER_OUTLINE;
use super::variables::COLOR_BODY_BACKGROUND;
use super::variables::COLOR_BODY_FONT;

pub fn style_field_text_input(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let base = text_input::default(theme, status);

    text_input::Style {
        background: Background::Color(COLOR_BODY_BACKGROUND),
        border: BORDER_OUTLINE,
        icon: COLOR_BODY_FONT,
        placeholder: base.placeholder,
        value: COLOR_BODY_FONT,
        ..base
    }
}
