use iced::Center;
use iced::Element;
use iced::Fill;
use iced::alignment::Horizontal;
use iced::widget::button;
use iced::widget::column;
use iced::widget::container;
use iced::widget::pick_list;
use iced::widget::row;
use iced::widget::text;
use iced::widget::text_input;

use crate::font::BOLD;
use crate::message::Message;
use crate::model_preset::ModelPreset;
use crate::start_cluster_config_data::StartClusterConfigData;
use crate::style_button_primary::style_button_primary;
use crate::style_field_container::style_field_container;
use crate::style_field_pick_list::style_field_pick_list;
use crate::style_field_pick_list_menu::style_field_pick_list_menu;
use crate::style_field_text_input::style_field_text_input;
use crate::variables::FONT_SIZE_L2;
use crate::variables::SPACING_2X;
use crate::variables::SPACING_BASE;
use crate::variables::SPACING_HALF;

pub fn view_start_cluster_config<'content>(
    data: &'content StartClusterConfigData,
) -> Element<'content, Message> {
    let available_models = ModelPreset::available_presets();

    let confirm_button = if data.starting {
        button(text("Starting...").font(BOLD))
            .padding([SPACING_HALF, SPACING_BASE])
            .style(style_button_primary)
    } else if data.selected_model.is_some()
        && !data.balancer_address.is_empty()
        && !data.inference_address.is_empty()
    {
        button(text("Start a cluster").font(BOLD))
            .padding([SPACING_HALF, SPACING_BASE])
            .style(style_button_primary)
            .on_press(Message::Confirm)
    } else {
        button(text("Start a cluster").font(BOLD))
            .padding([SPACING_HALF, SPACING_BASE])
            .style(style_button_primary)
    };

    let cancel_button = button(text("Cancel").font(BOLD))
        .style(button::text)
        .on_press(Message::Cancel);

    let mut content = column![
        container(text("Start a cluster").size(FONT_SIZE_L2).font(BOLD))
            .padding([0.0, SPACING_BASE]),
        column![
            container(text("Balancer address").font(BOLD)).padding([0.0, SPACING_BASE]),
            container(
                text_input("IP:port", &data.balancer_address)
                    .on_input(Message::SetBalancerAddress)
                    .padding(SPACING_BASE)
                    .style(style_field_text_input),
            )
            .width(300)
            .style(style_field_container),
        ]
        .spacing(SPACING_HALF),
        column![
            container(text("Inference address").font(BOLD)).padding([0.0, SPACING_BASE]),
            container(
                text_input("IP:port", &data.inference_address)
                    .on_input(Message::SetInferenceAddress)
                    .padding(SPACING_BASE)
                    .style(style_field_text_input),
            )
            .width(300)
            .style(style_field_container),
        ]
        .spacing(SPACING_HALF),
        column![
            container(text("Select a model").font(BOLD)).padding([0.0, SPACING_BASE]),
            container(
                pick_list(
                    available_models,
                    data.selected_model.as_ref(),
                    Message::SelectModel,
                )
                .width(Fill)
                .padding(SPACING_BASE)
                .style(style_field_pick_list)
                .menu_style(style_field_pick_list_menu),
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
