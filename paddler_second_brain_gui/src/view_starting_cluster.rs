use iced::Element;
use iced::widget::text;

use crate::message::Message;

pub fn view_starting_cluster() -> Element<'static, Message> {
    text("Starting cluster...").into()
}
