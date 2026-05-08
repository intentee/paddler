pub mod pyjinja_tojson;
pub mod raise_exception;

use anyhow::Result;
use minijinja::Environment;
use minijinja_contrib::pycompat::unknown_method_callback;
use paddler_types::chat_template::ChatTemplate;
use serde::ser::Serialize;

use self::pyjinja_tojson::pyjinja_tojson;
use self::raise_exception::raise_exception;

const CHAT_TEMPLATE_NAME: &str = "chat_template";

pub struct ChatTemplateRenderer {
    minijinja_env: Environment<'static>,
}

impl ChatTemplateRenderer {
    pub fn new(ChatTemplate { content }: ChatTemplate) -> Result<Self> {
        let mut minijinja_env = Environment::new();

        minijinja_env.add_function("raise_exception", raise_exception);
        minijinja_env.add_template_owned(CHAT_TEMPLATE_NAME, content)?;
        minijinja_env.set_unknown_method_callback(unknown_method_callback);

        minijinja_contrib::add_to_environment(&mut minijinja_env);
        minijinja_env.add_filter("tojson", pyjinja_tojson);

        Ok(Self { minijinja_env })
    }

    pub fn render<TContext: Serialize>(&self, context: TContext) -> Result<String> {
        Ok(self
            .minijinja_env
            .get_template(CHAT_TEMPLATE_NAME)?
            .render(context)?)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use anyhow::Result;
    use minijinja::context;
    use paddler_types::chat_template::ChatTemplate;
    use paddler_types::chat_template_message::ChatTemplateMessage;
    use paddler_types::chat_template_message_content::ChatTemplateMessageContent;

    use crate::chat_template_renderer::ChatTemplateRenderer;

    #[test]
    fn new_with_valid_template_succeeds() {
        let template = ChatTemplate {
            content: "Hello {{ name }}!".to_owned(),
        };

        assert!(ChatTemplateRenderer::new(template).is_ok());
    }

    #[test]
    fn new_with_invalid_template_fails() {
        let template = ChatTemplate {
            content: "{% if unclosed %}".to_owned(),
        };

        assert!(ChatTemplateRenderer::new(template).is_err());
    }

    #[test]
    fn render_produces_expected_output() -> Result<()> {
        let template = ChatTemplate {
            content: "Hello {{ name }}!".to_owned(),
        };
        let renderer = ChatTemplateRenderer::new(template)?;
        let mut context = HashMap::new();
        context.insert("name", "world");

        let result = renderer.render(context)?;

        assert_eq!(result, "Hello world!");

        Ok(())
    }

    #[test]
    fn renders_messages_loop_with_roles() -> Result<()> {
        let template = ChatTemplate {
            content: "{% for message in messages %}{{ message.role }}:{{ message.content }}\n{% endfor %}".to_owned(),
        };
        let renderer = ChatTemplateRenderer::new(template)?;
        let messages = vec![
            ChatTemplateMessage {
                content: ChatTemplateMessageContent::Text("hi".to_owned()),
                role: "user".to_owned(),
            },
            ChatTemplateMessage {
                content: ChatTemplateMessageContent::Text("hello".to_owned()),
                role: "assistant".to_owned(),
            },
        ];

        let result = renderer.render(context! { messages => messages })?;

        assert_eq!(result, "user:hi\nassistant:hello\n");

        Ok(())
    }

    #[test]
    fn add_generation_prompt_branch_changes_output() -> Result<()> {
        let template = ChatTemplate {
            content: "A{% if add_generation_prompt %}B{% endif %}".to_owned(),
        };
        let renderer = ChatTemplateRenderer::new(template)?;

        let with_prompt = renderer.render(context! { add_generation_prompt => true })?;
        let without_prompt = renderer.render(context! { add_generation_prompt => false })?;

        assert_eq!(with_prompt, "AB");
        assert_eq!(without_prompt, "A");

        Ok(())
    }

    #[test]
    fn registers_pyjinja_tojson_filter() -> Result<()> {
        let template = ChatTemplate {
            content: "{{ value | tojson(ensure_ascii=False) }}".to_owned(),
        };
        let renderer = ChatTemplateRenderer::new(template)?;

        let result = renderer.render(context! { value => "café" })?;

        assert_eq!(result, "\"café\"");

        Ok(())
    }

    #[test]
    fn registers_raise_exception_function() -> Result<()> {
        let template = ChatTemplate {
            content: "{{ raise_exception('boom') }}".to_owned(),
        };
        let template_renderer = ChatTemplateRenderer::new(template)?;

        let err = template_renderer
            .render(context! {})
            .err()
            .ok_or_else(|| anyhow::anyhow!("expected Err, got Ok"))?;
        let error_message = err.to_string();

        if !error_message.contains("boom") {
            return Err(anyhow::anyhow!(
                "raise_exception must surface its message; got: {error_message}"
            ));
        }

        Ok(())
    }
}
