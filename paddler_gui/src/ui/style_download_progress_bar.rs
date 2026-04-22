use iced::Background;
use iced::Border;
use iced::Theme;
use iced::widget::progress_bar;

use super::variables::COLOR_BODY_BACKGROUND;
use super::variables::COLOR_BORDER;

pub fn style_download_progress_bar(_theme: &Theme) -> progress_bar::Style {
    progress_bar::Style {
        background: Background::Color(COLOR_BODY_BACKGROUND),
        bar: Background::Color(COLOR_BORDER),
        border: Border {
            color: COLOR_BORDER,
            width: 2.0,
            radius: 0.into(),
        },
    }
}
