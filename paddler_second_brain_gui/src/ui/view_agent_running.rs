use iced::Center;
use iced::Element;
use iced::Fill;
use iced::widget::button;
use iced::widget::column;
use iced::widget::container;
use iced::widget::row;
use iced::widget::svg;
use iced::widget::svg::Handle as SvgHandle;
use iced::widget::text;

use super::font::BOLD;
use super::font::REGULAR;
use super::style_button_disconnect::style_button_disconnect;
use super::variables::FONT_SIZE_L2;
use super::variables::SPACING_2X;
use super::variables::SPACING_BASE;
use super::variables::SPACING_HALF;
use super::view_agent_card::view_agent_card;
use crate::agent_running_data::AgentRunningData;
use crate::message::Message;

pub fn view_agent_running<'content>(
    data: &'content AgentRunningData,
) -> Element<'content, Message> {
    let stop_icon = svg(SvgHandle::from_memory(
        include_bytes!("../../../resources/icons/stop.svg").as_slice(),
    ))
    .width(16)
    .height(16);

    let disconnect_button = button(
        row![stop_icon, text("Disconnect").font(BOLD)]
            .spacing(SPACING_HALF)
            .align_y(Center),
    )
    .padding([SPACING_HALF, SPACING_BASE])
    .style(style_button_disconnect)
    .on_press(Message::Disconnect);

    let connection_status = if data.connected {
        text(format!(
            "Connected to the balancer at {}",
            data.cluster_address
        ))
        .font(REGULAR)
    } else {
        text("Connecting to the balancer...").font(REGULAR)
    };

    let status_row = container(
        row![container(connection_status).width(Fill), disconnect_button,].align_y(Center),
    )
    .padding([0.0, SPACING_BASE]);

    column![
        container(text("Your agent").size(FONT_SIZE_L2).font(BOLD)).padding([0.0, SPACING_BASE]),
        container(text("Agent details").font(BOLD)).padding([0.0, SPACING_BASE]),
        view_agent_card(&data.snapshot),
        status_row,
    ]
    .spacing(SPACING_2X)
    .into()
}
