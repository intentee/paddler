use iced::Background;
use iced::Border;
use iced::Theme;
use iced::widget::container;

use super::variables::COLOR_BODY_BACKGROUND;
use super::variables::COLOR_BORDER;

pub fn style_card_container(theme: &Theme) -> container::Style {
    let base = container::transparent(theme);

    container::Style {
        background: Some(Background::Color(COLOR_BODY_BACKGROUND)),
        border: Border {
            color: COLOR_BORDER,
            width: 2.0,
            radius: 0.into(),
        },
        ..base
    }
}
