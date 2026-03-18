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

    let mut content = column![text("Your cluster").size(20), text(agent_label)].spacing(10);

    for interface in &data.network_interfaces {
        let address = format!("{}:{}", interface.ip_address, data.management_port);

        content = content
            .push(row![text(interface.interface_name.to_string()), text(address),].spacing(10));
    }

    if data.network_interfaces.is_empty() {
        content = content.push(text("No network interfaces detected"));
    }

    content = content.push(button("Stop cluster").on_press(Message::Stop));

    content.into()
}
