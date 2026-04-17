use iced::Background;
use iced::Border;
use iced::Color;
use iced::Theme;
use iced::widget::container;

pub fn style_status_indicator(theme: &Theme) -> container::Style {
    let base = container::transparent(theme);

    container::Style {
        background: Some(Background::Color(Color::from_rgb8(0xEE, 0xFF, 0xEE))),
        border: Border {
            color: Color::from_rgb8(0xCC, 0xDD, 0xCC),
            width: 2.0,
            radius: 8.into(),
        },
        ..base
    }
}
