use iced::Element;
use iced::widget::button;
use iced::widget::column;
use iced::widget::pick_list;
use iced::widget::text;
use iced::widget::toggler;

use crate::message::Message;
use crate::model_preset::ModelPreset;
use crate::start_cluster_config_data::StartClusterConfigData;

pub fn view_start_cluster_config<'content>(
    data: &'content StartClusterConfigData,
) -> Element<'content, Message> {
    let available_models = ModelPreset::available_presets();

    let confirm_button = if data.selected_model.is_some() {
        button("Start a cluster").on_press(Message::Confirm)
    } else {
        button("Start a cluster")
    };

    let mut content = column![
        button("Back").on_press(Message::Cancel),
        text("Select a model"),
        pick_list(
            available_models,
            data.selected_model.as_ref(),
            Message::SelectModel,
        ),
        toggler(data.run_agent_locally)
            .label("Run an agent on your computer")
            .on_toggle(Message::ToggleRunAgentLocally),
        confirm_button,
    ]
    .spacing(10);

    if let Some(error) = &data.error {
        content = content.push(text(error.clone()));
    }

    content.into()
}
