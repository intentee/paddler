pub mod pyjinja_tojson;
pub mod raise_exception;

use crate::chat_template::ChatTemplate;
use anyhow::Context as _;
use anyhow::Result;
use minijinja::Environment;
use minijinja_contrib::pycompat::unknown_method_callback;
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
            .get_template(CHAT_TEMPLATE_NAME)
            .context("chat template is not registered in the rendering environment")?
            .render(context)?)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::chat_template::ChatTemplate;
    use crate::chat_template_message::ChatTemplateMessage;
    use crate::chat_template_message_content::ChatTemplateMessageContent;
    use minijinja::context;

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
    fn render_produces_expected_output() {
        let template = ChatTemplate {
            content: "Hello {{ name }}!".to_owned(),
        };
        let renderer = ChatTemplateRenderer::new(template).unwrap();
        let mut context = HashMap::new();
        context.insert("name", "world");

        let result = renderer.render(context).unwrap();

        assert_eq!(result, "Hello world!");
    }

    #[test]
    fn renders_messages_loop_with_roles() {
        let template = ChatTemplate {
            content: "{% for message in messages %}{{ message.role }}:{{ message.content }}\n{% endfor %}".to_owned(),
        };
        let renderer = ChatTemplateRenderer::new(template).unwrap();
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

        let result = renderer.render(context! { messages => messages }).unwrap();

        assert_eq!(result, "user:hi\nassistant:hello\n");
    }

    #[test]
    fn add_generation_prompt_branch_changes_output() {
        let template = ChatTemplate {
            content: "A{% if add_generation_prompt %}B{% endif %}".to_owned(),
        };
        let renderer = ChatTemplateRenderer::new(template).unwrap();

        let with_prompt = renderer
            .render(context! { add_generation_prompt => true })
            .unwrap();
        let without_prompt = renderer
            .render(context! { add_generation_prompt => false })
            .unwrap();

        assert_eq!(with_prompt, "AB");
        assert_eq!(without_prompt, "A");
    }

    #[test]
    fn registers_pyjinja_tojson_filter() {
        let template = ChatTemplate {
            content: "{{ value | tojson(ensure_ascii=False) }}".to_owned(),
        };
        let renderer = ChatTemplateRenderer::new(template).unwrap();

        let result = renderer.render(context! { value => "café" }).unwrap();

        assert_eq!(result, "\"café\"");
    }

    #[test]
    fn registers_raise_exception_function() {
        let template = ChatTemplate {
            content: "{{ raise_exception('boom') }}".to_owned(),
        };
        let template_renderer = ChatTemplateRenderer::new(template).unwrap();

        let render_error = template_renderer
            .render(context! {})
            .expect_err("raise_exception must turn rendering into an error");
        let error_message = render_error.to_string();

        assert!(
            error_message.contains("boom"),
            "raise_exception must surface its message; got: {error_message}"
        );
    }

    #[test]
    fn render_fails_when_template_is_not_registered() {
        let template = ChatTemplate {
            content: "Hello {{ name }}!".to_owned(),
        };
        let mut renderer = ChatTemplateRenderer::new(template).unwrap();

        renderer
            .minijinja_env
            .remove_template(super::CHAT_TEMPLATE_NAME);

        let render_error = renderer
            .render(context! {})
            .expect_err("rendering must fail when the template is missing");
        let error_message = render_error.to_string();

        assert_eq!(
            error_message,
            "chat template is not registered in the rendering environment"
        );
    }
}
