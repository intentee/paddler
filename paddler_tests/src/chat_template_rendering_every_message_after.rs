use paddler_messaging::chat_template::ChatTemplate;

const EVERY_MESSAGE_AND_THE_GENERATION_PROMPT: &str = "{%- for message in messages -%}{{ message['role'] }}: {% if message['content'] is string %}{{ message['content'] }}{% else %}{% for part in message['content'] %}{{ part['text'] }}{% endfor %}{% endif %}\n{% endfor -%}{% if add_generation_prompt %}assistant: {% endif %}";

#[must_use]
pub fn chat_template_rendering_every_message_after(prefix: &str) -> ChatTemplate {
    ChatTemplate {
        content: format!("{prefix}{EVERY_MESSAGE_AND_THE_GENERATION_PROMPT}"),
    }
}
