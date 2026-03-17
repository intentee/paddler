use iced::Element;
use iced::widget::button;
use iced::widget::column;
use iced::widget::pick_list;
use iced::widget::text;
use iced::widget::toggler;

use crate::message::Message;
use crate::start_cluster_config_data::StartClusterConfigData;

const AVAILABLE_MODELS: &[&str] = &[
    "llama-3.2-1b",
    "llama-3.2-3b",
    "llama-3.1-8b",
    "mistral-7b",
    "phi-3-mini",
];

pub fn view_start_cluster_config<'content>(
    data: &'content StartClusterConfigData,
) -> Element<'content, Message> {
    column![
        button("Back").on_press(Message::Cancel),
        text("Select a model"),
        pick_list(
            AVAILABLE_MODELS,
            data.selected_model.as_deref(),
            |model: &str| Message::SelectModel(model.to_owned()),
        ),
        toggler(data.run_agent_locally)
            .label("Run an agent on your computer")
            .on_toggle(Message::ToggleRunAgentLocally),
        button("Start a cluster").on_press(Message::Confirm),
    ]
    .spacing(10)
    .into()
}
