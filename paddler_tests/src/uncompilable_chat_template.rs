use paddler_messaging::chat_template::ChatTemplate;

use crate::constants::UNCOMPILABLE_CHAT_TEMPLATE_CONTENT;

#[must_use]
pub fn uncompilable_chat_template() -> ChatTemplate {
    ChatTemplate {
        content: UNCOMPILABLE_CHAT_TEMPLATE_CONTENT.to_owned(),
    }
}
