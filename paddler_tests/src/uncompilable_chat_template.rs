use paddler_messaging::chat_template::ChatTemplate;

pub const UNCOMPILABLE_CHAT_TEMPLATE_CONTENT: &str = "{{invalid jinja template";

#[must_use]
pub fn uncompilable_chat_template() -> ChatTemplate {
    ChatTemplate {
        content: UNCOMPILABLE_CHAT_TEMPLATE_CONTENT.to_owned(),
    }
}
