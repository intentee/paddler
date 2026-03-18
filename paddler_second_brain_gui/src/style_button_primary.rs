use iced::Background;
use iced::Border;
use iced::widget::button;

use crate::variables::COLOR_BODY_BACKGROUND;
use crate::variables::COLOR_BORDER;

pub fn style_button_primary(theme: &iced::Theme, status: button::Status) -> button::Style {
    let base = button::primary(theme, status);

    button::Style {
        background: Some(Background::Color(COLOR_BORDER)),
        text_color: COLOR_BODY_BACKGROUND,
        border: Border {
            color: COLOR_BORDER,
            width: 0.0,
            radius: 0.into(),
        },
        ..base
    }
}
