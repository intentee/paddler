use iced::Center;
use iced::Element;
use iced::alignment::Horizontal;
use iced::widget::button;
use iced::widget::column;
use iced::widget::container;
use iced::widget::row;
use iced::widget::text;
use iced::widget::text_input;

use super::font::BOLD;
use super::font::REGULAR;
use super::style_button_primary::style_button_primary;
use super::style_field_container::style_field_container;
use super::style_field_text_input::style_field_text_input;
use super::variables::COLOR_ERROR;
use super::variables::FONT_SIZE_L2;
use super::variables::SPACING_2X;
use super::variables::SPACING_BASE;
use super::variables::SPACING_HALF;
use crate::join_cluster_config_data::JoinClusterConfigData;
use crate::message::Message;

pub fn view_join_cluster_config(data: &JoinClusterConfigData) -> Element<'_, Message> {
    let confirm_button = button(text("Connect").font(BOLD))
        .padding([SPACING_HALF, SPACING_BASE])
        .style(style_button_primary)
        .on_press(Message::Connect);

    let cancel_button = button(text("Cancel").font(BOLD))
        .style(button::text)
        .on_press(Message::Cancel);

    let mut cluster_address_field = column![
        container(text("Cluster address").font(BOLD)).padding([0.0, SPACING_BASE]),
        container(
            text_input("IP:port", &data.cluster_address)
                .on_input(Message::SetClusterAddress)
                .padding(SPACING_BASE)
                .style(style_field_text_input),
        )
        .width(400)
        .style(style_field_container),
    ]
    .spacing(SPACING_HALF);

    if let Some(error) = &data.cluster_address_error {
        cluster_address_field = cluster_address_field.push(
            container(text(error.clone()).font(REGULAR).color(COLOR_ERROR))
                .width(400)
                .padding([0.0, SPACING_BASE]),
        );
    }

    let mut slots_field = column![
        container(text("Slots").font(BOLD)).padding([0.0, SPACING_BASE]),
        container(
            text_input("e.g. 1", &data.slots_count)
                .on_input(Message::SetSlotsCount)
                .padding(SPACING_BASE)
                .style(style_field_text_input),
        )
        .width(400)
        .style(style_field_container),
    ]
    .spacing(SPACING_HALF);

    if let Some(error) = &data.slots_error {
        slots_field = slots_field.push(
            container(text(error.clone()).font(REGULAR).color(COLOR_ERROR))
                .width(400)
                .padding([0.0, SPACING_BASE]),
        );
    }

    column![
        container(text("Join a cluster").size(FONT_SIZE_L2).font(BOLD))
            .padding([0.0, SPACING_BASE]),
        cluster_address_field,
        column![
            container(text("Agent name (optional)").font(BOLD)).padding([0.0, SPACING_BASE]),
            container(
                text_input("my-agent", &data.agent_name)
                    .on_input(Message::SetAgentName)
                    .padding(SPACING_BASE)
                    .style(style_field_text_input),
            )
            .width(400)
            .style(style_field_container),
        ]
        .spacing(SPACING_HALF),
        slots_field,
        container(
            row![cancel_button, confirm_button]
                .align_y(Center)
                .spacing(SPACING_BASE),
        )
        .width(400)
        .align_x(Horizontal::Right),
    ]
    .spacing(SPACING_2X)
    .into()
}
