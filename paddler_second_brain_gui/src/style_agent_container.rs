use iced::Background;
use iced::Border;
use iced::Theme;
use iced::widget::container;

use crate::variables::COLOR_AGENT_BACKGROUND;
use crate::variables::COLOR_BORDER;

pub fn style_agent_container(theme: &Theme) -> container::Style {
    let base = container::transparent(theme);

    container::Style {
        background: Some(Background::Color(COLOR_AGENT_BACKGROUND)),
        border: Border {
            color: COLOR_BORDER,
            width: 2.0,
            radius: 0.into(),
        },
        ..base
    }
}
