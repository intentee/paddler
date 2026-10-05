use iced::Background;
use iced::Theme;
use iced::widget::pick_list;

use super::variables::BORDER_OUTLINE;
use super::variables::COLOR_BODY_BACKGROUND;
use super::variables::COLOR_BODY_FONT;

pub fn style_field_pick_list(theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let base = pick_list::default(theme, status);

    pick_list::Style {
        text_color: COLOR_BODY_FONT,
        placeholder_color: base.placeholder_color,
        handle_color: COLOR_BODY_FONT,
        background: Background::Color(COLOR_BODY_BACKGROUND),
        border: BORDER_OUTLINE,
    }
}
