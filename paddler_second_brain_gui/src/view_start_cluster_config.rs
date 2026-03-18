use iced::Element;
use iced::widget::button;
use iced::widget::column;
use iced::widget::pick_list;
use iced::widget::text;
use iced::widget::text_input;

use crate::message::Message;
use crate::model_preset::ModelPreset;
use crate::start_cluster_config_data::StartClusterConfigData;

pub fn view_start_cluster_config<'content>(
    data: &'content StartClusterConfigData,
) -> Element<'content, Message> {
    let available_models = ModelPreset::available_presets();

    let confirm_button = if data.starting {
        button("Starting...")
    } else if data.selected_model.is_some() && !data.bind_address.is_empty() {
        button("Start a cluster").on_press(Message::Confirm)
    } else {
        button("Start a cluster")
    };

    let mut content = column![
        button("Back").on_press(Message::Cancel),
        text("Balancer address"),
        text_input("IP address", &data.bind_address).on_input(Message::SetBindAddress),
        text("Balancer port"),
        text_input("Port", &data.bind_port).on_input(Message::SetBindPort),
        text("Select a model"),
        pick_list(
            available_models,
            data.selected_model.as_ref(),
            Message::SelectModel,
        ),
        confirm_button,
    ]
    .spacing(10);

    if let Some(error) = &data.error {
        content = content.push(text(error.clone()));
    }

    content.into()
}
