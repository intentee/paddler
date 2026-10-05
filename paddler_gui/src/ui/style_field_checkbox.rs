use iced::Background;
use iced::Border;
use iced::Theme;
use iced::widget::checkbox;

use super::variables::COLOR_BODY_BACKGROUND;
use super::variables::COLOR_BODY_FONT;
use super::variables::COLOR_BORDER;

pub fn style_field_checkbox(_theme: &Theme, status: checkbox::Status) -> checkbox::Style {
    let is_checked = matches!(
        status,
        checkbox::Status::Active { is_checked: true }
            | checkbox::Status::Hovered { is_checked: true }
            | checkbox::Status::Disabled { is_checked: true }
    );

    let (background, icon_color) = if is_checked {
        (COLOR_BORDER, COLOR_BODY_BACKGROUND)
    } else {
        (COLOR_BODY_BACKGROUND, COLOR_BODY_FONT)
    };

    checkbox::Style {
        background: Background::Color(background),
        border: Border {
            color: COLOR_BORDER,
            width: 2.0,
            radius: 0.into(),
        },
        icon_color,
        text_color: Some(COLOR_BODY_FONT),
    }
}
