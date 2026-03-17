use iced::Element;
use iced::widget::button;
use iced::widget::column;
use iced::widget::text;

use crate::message::Message;
use crate::running_cluster_data::RunningClusterData;

pub fn view_running_cluster<'content>(
    data: &'content RunningClusterData,
) -> Element<'content, Message> {
    column![
        text(format!("Cluster is running at {}", data.cluster_address)),
        button("Stop cluster").on_press(Message::Stop),
    ]
    .spacing(10)
    .into()
}
