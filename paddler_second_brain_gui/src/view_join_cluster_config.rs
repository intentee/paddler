use iced::Center;
use iced::Element;
use iced::alignment::Horizontal;
use iced::widget::button;
use iced::widget::column;
use iced::widget::container;
use iced::widget::row;
use iced::widget::text;
use iced::widget::text_input;

use crate::font::BOLD;
use crate::join_cluster_config_data::JoinClusterConfigData;
use crate::message::Message;
use crate::style_button_primary::style_button_primary;
use crate::style_field_container::style_field_container;
use crate::style_field_text_input::style_field_text_input;
use crate::variables::FONT_SIZE_L2;
use crate::variables::SPACING_2X;
use crate::variables::SPACING_BASE;
use crate::variables::SPACING_HALF;

pub fn view_join_cluster_config<'content>(
    data: &'content JoinClusterConfigData,
) -> Element<'content, Message> {
    let is_valid_slots = data
        .slots_count
        .parse::<i32>()
        .map(|slots| slots > 0)
        .unwrap_or(false);

    let confirm_button =
        if !data.cluster_address.is_empty() && !data.agent_name.is_empty() && is_valid_slots {
            button(text("Connect").font(BOLD))
                .padding([SPACING_HALF, SPACING_BASE])
                .style(style_button_primary)
                .on_press(Message::Connect)
        } else {
            button(text("Connect").font(BOLD))
                .padding([SPACING_HALF, SPACING_BASE])
                .style(style_button_primary)
        };

    let cancel_button = button(text("Cancel").font(BOLD))
        .style(button::text)
        .on_press(Message::Cancel);

    let mut content = column![
        container(text("Join a cluster").size(FONT_SIZE_L2).font(BOLD))
            .padding([0.0, SPACING_BASE]),
        column![
            container(text("Cluster address").font(BOLD)).padding([0.0, SPACING_BASE]),
            container(
                text_input("", &data.cluster_address)
                    .on_input(Message::SetClusterAddress)
                    .padding(SPACING_BASE)
                    .style(style_field_text_input),
            )
            .width(300)
            .style(style_field_container),
        ]
        .spacing(SPACING_HALF),
        column![
            container(text("Agent name").font(BOLD)).padding([0.0, SPACING_BASE]),
            container(
                text_input("my-agent", &data.agent_name)
                    .on_input(Message::SetAgentName)
                    .padding(SPACING_BASE)
                    .style(style_field_text_input),
            )
            .width(300)
            .style(style_field_container),
        ]
        .spacing(SPACING_HALF),
        column![
            container(text("Slots").font(BOLD)).padding([0.0, SPACING_BASE]),
            container(
                text_input("e.g. 1", &data.slots_count)
                    .on_input(Message::SetSlotsCount)
                    .padding(SPACING_BASE)
                    .style(style_field_text_input),
            )
            .width(300)
            .style(style_field_container),
        ]
        .spacing(SPACING_HALF),
        container(
            row![cancel_button, confirm_button]
                .align_y(Center)
                .spacing(SPACING_BASE),
        )
        .width(300)
        .align_x(Horizontal::Right),
    ]
    .spacing(SPACING_2X);

    if let Some(error) = &data.error {
        content = content.push(text(error.clone()));
    }

    content.into()
}
