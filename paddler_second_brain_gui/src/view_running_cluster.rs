use iced::Element;
use iced::widget::button;
use iced::widget::column;
use iced::widget::row;
use iced::widget::text;

use crate::message::Message;
use crate::running_cluster_data::RunningClusterData;

pub fn view_running_cluster<'content>(
    data: &'content RunningClusterData,
) -> Element<'content, Message> {
    let agent_label = match data.agent_count {
        1 => "1 agent connected".to_string(),
        count => format!("{count} agents connected"),
    };

    let mut content = column![
        text("Your cluster").size(20),
        text(agent_label),
        row![
            text(data.cluster_address.clone()),
            button("Copy").on_press(Message::CopyToClipboard(data.cluster_address.clone())),
        ]
        .spacing(10),
    ]
    .spacing(10);

    content = content.push(if data.stopping {
        button("Stopping...")
    } else {
        button("Stop cluster").on_press(Message::Stop)
    });

    content.into()
}
