use iced::Background;
use iced::Theme;
use iced::widget::container;

use super::variables::BORDER_OUTLINE;
use super::variables::COLOR_BODY_BACKGROUND;

pub fn style_card_container(theme: &Theme) -> container::Style {
    let base = container::transparent(theme);

    container::Style {
        background: Some(Background::Color(COLOR_BODY_BACKGROUND)),
        border: BORDER_OUTLINE,
        ..base
    }
}
