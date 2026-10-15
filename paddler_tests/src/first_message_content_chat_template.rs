use paddler_messaging::chat_template::ChatTemplate;

#[must_use]
pub fn first_message_content_chat_template() -> ChatTemplate {
    ChatTemplate {
        content: "{{ messages[0].content }}".to_owned(),
    }
}
