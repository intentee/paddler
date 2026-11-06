use paddler_messaging::chat_template::ChatTemplate;

#[must_use]
pub fn first_byte_of_the_first_message_chat_template() -> ChatTemplate {
    ChatTemplate {
        content: "{{ messages[0].content[:1] }}".to_owned(),
    }
}
