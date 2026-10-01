use iced::Element;
use iced::widget::container;
use iced::widget::text;

use super::font::REGULAR;
use super::variables::COLOR_ERROR;
use super::variables::SPACING_BASE;

pub fn view_error_message<TMessage: 'static>(error: &str) -> Element<'static, TMessage> {
    container(text(error.to_owned()).font(REGULAR).color(COLOR_ERROR))
        .padding([0.0, SPACING_BASE])
        .into()
}
