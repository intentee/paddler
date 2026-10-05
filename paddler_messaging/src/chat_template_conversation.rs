use crate::chat_template_message::ChatTemplateMessage;
use crate::image_url::ImageUrl;

#[derive(Debug, Eq, PartialEq)]
pub struct ChatTemplateConversation {
    pub image_urls: Vec<ImageUrl>,
    pub messages: Vec<ChatTemplateMessage>,
}
