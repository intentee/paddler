use paddler_messaging::chat_template::ChatTemplate;

pub const UNCOMPILABLE_CHAT_TEMPLATE_CONTENT: &str = "{{invalid jinja template";
pub const UNCOMPILABLE_CHAT_TEMPLATE_PARSE_FAILURE: &str =
    "parser: Parser Error: Expected }} (Got jinja)\n...{{invalid jinja template...\n             ^";

#[must_use]
pub fn uncompilable_chat_template() -> ChatTemplate {
    ChatTemplate {
        content: UNCOMPILABLE_CHAT_TEMPLATE_CONTENT.to_owned(),
    }
}
