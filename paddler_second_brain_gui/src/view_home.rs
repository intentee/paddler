use iced::Element;
use iced::widget::button;
use iced::widget::column;

use crate::message::Message;

pub fn view_home() -> Element<'static, Message> {
    column![
        button("Start a cluster").on_press(Message::StartCluster),
        button("Join a cluster"),
    ]
    .spacing(10)
    .into()
}
