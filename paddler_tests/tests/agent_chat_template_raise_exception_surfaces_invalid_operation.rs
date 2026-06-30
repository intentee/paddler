#![cfg(feature = "tests_that_use_llms")]

use anyhow::Context as _;
use anyhow::Result;
use minijinja::Error as MinijinjaError;
use minijinja::ErrorKind;
use minijinja::context;
use paddler_agent::chat_template_renderer::ChatTemplateRenderer;
use paddler_messaging::chat_template::ChatTemplate;

#[test]
fn agent_chat_template_raise_exception_surfaces_invalid_operation() -> Result<()> {
    let template_renderer = ChatTemplateRenderer::new(ChatTemplate {
        content: "{{ raise_exception('boom') }}".to_owned(),
    })?;

    let render_error = template_renderer
        .render(context! {})
        .err()
        .context("raise_exception must turn rendering into an error")?;
    let minijinja_error = render_error
        .downcast_ref::<MinijinjaError>()
        .context("the render failure must surface as a minijinja error")?;

    assert_eq!(minijinja_error.kind(), ErrorKind::InvalidOperation);

    Ok(())
}
