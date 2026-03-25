use anyhow::Result;
use minijinja::Environment;
use minijinja::Error;
use minijinja::ErrorKind;
use minijinja_contrib::pycompat::unknown_method_callback;
use paddler_types::chat_template::ChatTemplate;
use serde::ser::Serialize;

const CHAT_TEMPLATE_NAME: &str = "chat_template";

// Known uses:
// https://huggingface.co/bartowski/Mistral-7B-Instruct-v0.3-GGUF
#[expect(
    clippy::needless_pass_by_value,
    reason = "minijinja function callback requires owned String"
)]
fn minijinja_raise_exception(message: String) -> std::result::Result<String, Error> {
    Err(Error::new::<String>(
        ErrorKind::InvalidOperation,
        format!("Model's chat template raised an exception: '{message}'"),
    ))
}

pub struct ChatTemplateRenderer {
    minijinja_env: Environment<'static>,
}

impl ChatTemplateRenderer {
    pub fn new(ChatTemplate { content }: ChatTemplate) -> Result<Self> {
        let mut minijinja_env = Environment::new();

        minijinja_env.add_function("raise_exception", minijinja_raise_exception);
        minijinja_env.add_template_owned(CHAT_TEMPLATE_NAME, content)?;
        minijinja_env.set_unknown_method_callback(unknown_method_callback);

        minijinja_contrib::add_to_environment(&mut minijinja_env);

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
    use paddler_types::chat_template::ChatTemplate;

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
}
