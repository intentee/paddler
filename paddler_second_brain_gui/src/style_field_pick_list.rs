use iced::Background;
use iced::Border;
use iced::Theme;
use iced::widget::pick_list;

use crate::variables::COLOR_BODY_BACKGROUND;
use crate::variables::COLOR_BODY_FONT;
use crate::variables::COLOR_BORDER;

pub fn style_field_pick_list(theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let base = pick_list::default(theme, status);

    pick_list::Style {
        text_color: COLOR_BODY_FONT,
        placeholder_color: base.placeholder_color,
        handle_color: COLOR_BODY_FONT,
        background: Background::Color(COLOR_BODY_BACKGROUND),
        border: Border {
            color: COLOR_BORDER,
            width: 2.0,
            radius: 0.into(),
        },
    }
}
