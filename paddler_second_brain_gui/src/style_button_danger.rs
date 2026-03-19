use iced::Background;
use iced::Border;
use iced::Color;
use iced::Theme;
use iced::widget::button;

pub fn style_button_danger(theme: &Theme, status: button::Status) -> button::Style {
    let base = button::primary(theme, status);

    let background_color = Color::from_rgb(
        0xCC as f32 / 255.0,
        0x33 as f32 / 255.0,
        0x33 as f32 / 255.0,
    );

    button::Style {
        background: Some(Background::Color(background_color)),
        text_color: Color::WHITE,
        border: Border {
            color: background_color,
            width: 0.0,
            radius: 0.into(),
        },
        ..base
    }
}
