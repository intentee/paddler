use iced::Background;
use iced::Border;
use iced::Color;
use iced::Theme;
use iced::widget::button;

use super::variables::COLOR_ERROR;

pub fn style_button_disconnect(theme: &Theme, status: button::Status) -> button::Style {
    let base = button::primary(theme, status);

    button::Style {
        background: Some(Background::Color(COLOR_ERROR)),
        text_color: Color::WHITE,
        border: Border {
            color: COLOR_ERROR,
            width: 0.0,
            radius: 0.into(),
        },
        ..base
    }
}
