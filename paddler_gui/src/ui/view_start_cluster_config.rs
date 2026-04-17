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

use super::font::BOLD;
use super::style_button_primary::style_button_primary;
use super::style_field_pick_list::style_field_pick_list;
use super::style_field_pick_list_menu::style_field_pick_list_menu;
use super::style_field_text_input::style_field_text_input;
use super::variables::FONT_SIZE_L2;
use super::variables::FORM_WIDTH;
use super::variables::SPACING_2X;
use super::variables::SPACING_BASE;
use super::variables::SPACING_HALF;
use super::view_form_field::view_form_field;
use crate::model_preset::ModelPreset;
use crate::start_cluster_config_data::StartClusterConfigData;
use crate::start_cluster_config_handler::Message;

pub fn view_start_cluster_config(data: &StartClusterConfigData) -> Element<'_, Message> {
    let available_models = ModelPreset::available_presets();

    let confirm_button = if data.starting {
        button(text("Starting...").font(BOLD))
            .padding([SPACING_HALF, SPACING_BASE])
            .style(style_button_primary)
    } else {
        button(text("Start a cluster").font(BOLD))
            .padding([SPACING_HALF, SPACING_BASE])
            .style(style_button_primary)
            .on_press(Message::Confirm)
    };

    let cancel_button = button(text("Cancel").font(BOLD))
        .style(button::text)
        .on_press(Message::Cancel);

    let cluster_address_input = text_input("IP:port", &data.cluster_address)
        .on_input(Message::SetClusterAddress)
        .padding(SPACING_BASE)
        .style(style_field_text_input)
        .into();

    let inference_address_input = text_input("IP:port", &data.inference_address)
        .on_input(Message::SetInferenceAddress)
        .padding(SPACING_BASE)
        .style(style_field_text_input)
        .into();

    let model_input = pick_list(
        available_models,
        data.selected_model.as_ref(),
        Message::SelectModel,
    )
    .width(Fill)
    .padding(SPACING_BASE)
    .style(style_field_pick_list)
    .menu_style(style_field_pick_list_menu)
    .into();

    column![
        container(text("Start a cluster").size(FONT_SIZE_L2).font(BOLD))
            .padding([0.0, SPACING_BASE]),
        container(
            column![
                view_form_field(
                    "Cluster address",
                    cluster_address_input,
                    data.cluster_address_error.as_ref()
                ),
                view_form_field(
                    "Inference address",
                    inference_address_input,
                    data.inference_address_error.as_ref()
                ),
                view_form_field("Select a model", model_input, data.model_error.as_ref()),
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
