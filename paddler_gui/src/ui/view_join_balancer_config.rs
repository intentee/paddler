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
use super::style_button_primary::style_button_primary;
use super::style_field_text_input::style_field_text_input;
use super::variables::FONT_SIZE_L2;
use super::variables::FORM_WIDTH;
use super::variables::SPACING_2X;
use super::variables::SPACING_BASE;
use super::variables::SPACING_HALF;
use super::view_form_field::view_form_field;
use crate::join_balancer_config_data::JoinBalancerConfigData;
use crate::join_balancer_config_handler::Message;

pub fn view_join_balancer_config(data: &JoinBalancerConfigData) -> Element<'_, Message> {
    let confirm_button = button(text("Connect").font(BOLD))
        .padding([SPACING_HALF, SPACING_BASE])
        .style(style_button_primary)
        .on_press(Message::Connect);

    let cancel_button = button(text("Cancel").font(BOLD))
        .style(button::text)
        .on_press(Message::Cancel);

    let balancer_address_input = text_input("IP:port", &data.balancer_address)
        .on_input(Message::SetBalancerAddress)
        .padding(SPACING_BASE)
        .style(style_field_text_input)
        .into();

    let agent_name_input = text_input("my-agent", &data.agent_name)
        .on_input(Message::SetAgentName)
        .padding(SPACING_BASE)
        .style(style_field_text_input)
        .into();

    let slots_input = text_input("e.g. 1", &data.slots_count)
        .on_input(Message::SetSlotsCount)
        .padding(SPACING_BASE)
        .style(style_field_text_input)
        .into();

    column![
        container(text("Join a cluster").size(FONT_SIZE_L2).font(BOLD))
            .padding([0.0, SPACING_BASE]),
        container(
            column![
                view_form_field(
                    "Cluster address",
                    balancer_address_input,
                    data.balancer_address_error.as_ref()
                ),
                view_form_field("Agent name (optional)", agent_name_input, None),
                view_form_field("Slots", slots_input, data.slots_error.as_ref()),
                container(
                    row![cancel_button, confirm_button]
                        .align_y(Center)
                        .spacing(SPACING_BASE),
                )
                .align_x(Horizontal::Right),
            ]
            .spacing(SPACING_2X),
        )
        .width(FORM_WIDTH),
    ]
    .spacing(SPACING_2X)
    .into()
}
