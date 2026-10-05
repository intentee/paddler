use iced::Background;
use iced::Theme;
use iced::widget::container;

use super::variables::BORDER_OUTLINE;
use super::variables::COLOR_AGENT_BACKGROUND;

pub fn style_agent_container(theme: &Theme) -> container::Style {
    let base = container::transparent(theme);

    container::Style {
        background: Some(Background::Color(COLOR_AGENT_BACKGROUND)),
        border: BORDER_OUTLINE,
        ..base
    }
}
