use iced::Element;
use iced::widget::text;

use crate::message::Message;

pub fn view_stopping_cluster() -> Element<'static, Message> {
    text("Stopping cluster...").into()
}
