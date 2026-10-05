use iced::Element;
use iced::widget::column;
use iced::widget::container;
use iced::widget::text;

use super::font::BOLD;
use super::style_field_container::style_field_container;
use super::variables::SPACING_BASE;
use super::variables::SPACING_HALF;
use super::view_error_message::view_error_message;
use crate::form_field_error::FormFieldError;

pub fn view_form_field<'element, TMessage: 'static>(
    label: &str,
    input: Element<'element, TMessage>,
    error: Option<&FormFieldError>,
) -> Element<'element, TMessage> {
    let mut field = column![
        container(text(label.to_owned()).font(BOLD)).padding([0.0, SPACING_BASE]),
        container(input).style(style_field_container),
    ]
    .spacing(SPACING_HALF);

    if let Some(error) = error {
        field = field.push(view_error_message(&error.to_string()));
    }

    field.into()
}
