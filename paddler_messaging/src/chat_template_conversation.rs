use llama_cpp_bindings_types::chat_message::ChatMessage;

use crate::image_url::ImageUrl;

#[derive(Debug, Eq, PartialEq)]
pub struct ChatTemplateConversation {
    pub image_urls: Vec<ImageUrl>,
    pub messages: Vec<ChatMessage>,
}
