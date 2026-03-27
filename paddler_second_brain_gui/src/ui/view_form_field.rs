use iced::Element;
use iced::widget::column;
use iced::widget::container;
use iced::widget::text;

use super::font::BOLD;
use super::font::REGULAR;
use super::style_field_container::style_field_container;
use super::variables::COLOR_ERROR;
use super::variables::SPACING_BASE;
use super::variables::SPACING_HALF;

pub fn view_form_field<'element, TMessage: 'static>(
    label: &str,
    input: Element<'element, TMessage>,
    error: Option<&String>,
) -> Element<'element, TMessage> {
    let mut field = column![
        container(text(label.to_owned()).font(BOLD)).padding([0.0, SPACING_BASE]),
        container(input).width(400).style(style_field_container),
    ]
    .spacing(SPACING_HALF);

    if let Some(error) = error {
        field = field.push(
            container(text(error.clone()).font(REGULAR).color(COLOR_ERROR))
                .width(400)
                .padding([0.0, SPACING_BASE]),
        );
    }

    field.into()
}
