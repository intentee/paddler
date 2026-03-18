use iced::Element;
use iced::widget::button;
use iced::widget::column;
use iced::widget::text;
use iced::widget::text_input;

use crate::join_cluster_config_data::JoinClusterConfigData;
use crate::message::Message;

pub fn view_join_cluster_config<'content>(
    data: &'content JoinClusterConfigData,
) -> Element<'content, Message> {
    let is_valid_slots = data
        .slots_count
        .parse::<i32>()
        .map(|slots| slots > 0)
        .unwrap_or(false);

    let connect_button = if !data.cluster_address.is_empty() && is_valid_slots {
        button("Connect").on_press(Message::Connect)
    } else {
        button("Connect")
    };

    let mut content = column![
        button("Back").on_press(Message::Cancel),
        text("Join a cluster"),
        text_input(
            "Cluster address (e.g. 192.168.1.5:8060)",
            &data.cluster_address,
        )
        .on_input(Message::SetClusterAddress),
        text_input("Slots (e.g. 1)", &data.slots_count).on_input(Message::SetSlotsCount),
        connect_button,
    ]
    .spacing(10);

    if let Some(error) = &data.error {
        content = content.push(text(error.clone()));
    }

    content.into()
}
