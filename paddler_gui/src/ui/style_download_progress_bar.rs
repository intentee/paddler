use iced::Background;
use iced::Theme;
use iced::widget::progress_bar;

use super::variables::BORDER_OUTLINE;
use super::variables::COLOR_BODY_BACKGROUND;
use super::variables::COLOR_BORDER;

pub const fn style_download_progress_bar(_theme: &Theme) -> progress_bar::Style {
    progress_bar::Style {
        background: Background::Color(COLOR_BODY_BACKGROUND),
        bar: Background::Color(COLOR_BORDER),
        border: BORDER_OUTLINE,
    }
}
