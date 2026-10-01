use iced::Center;
use iced::Element;
use iced::widget::button;
use iced::widget::row;
use iced::widget::svg;
use iced::widget::svg::Handle as SvgHandle;
use iced::widget::text;

use super::font::BOLD;
use super::style_button_disconnect::style_button_disconnect;
use super::variables::SPACING_BASE;
use super::variables::SPACING_HALF;

pub fn view_stop_button<TMessage: Clone + 'static>(
    label: &str,
    on_press: Option<TMessage>,
) -> Element<'static, TMessage> {
    let stop_icon = svg(SvgHandle::from_memory(
        include_bytes!("../../../resources/icons/stop.svg").as_slice(),
    ))
    .width(16)
    .height(16);

    button(
        row![stop_icon, text(label.to_owned()).font(BOLD)]
            .spacing(SPACING_HALF)
            .align_y(Center),
    )
    .padding([SPACING_HALF, SPACING_BASE])
    .style(style_button_disconnect)
    .on_press_maybe(on_press)
    .into()
}
