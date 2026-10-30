use std::path::Path;

use paddler_agent_text_generation::chat_template_renderer::ChatTemplateRenderer;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::chat_template_does_not_compile_params::ChatTemplateDoesNotCompileParams;
use paddler_messaging::agent_issue_params::model_path::ModelPath;

use crate::uncompilable_chat_template::UNCOMPILABLE_CHAT_TEMPLATE_CONTENT;
use crate::uncompilable_chat_template::uncompilable_chat_template;

#[must_use]
pub fn uncompilable_chat_template_issue(model_path: &Path) -> Option<AgentIssue> {
    ChatTemplateRenderer::new(uncompilable_chat_template())
        .err()
        .map(|compile_error| {
            AgentIssue::ChatTemplateDoesNotCompile(ChatTemplateDoesNotCompileParams {
                error: compile_error.to_string(),
                model_path: ModelPath::from(model_path),
                template_content: UNCOMPILABLE_CHAT_TEMPLATE_CONTENT.to_owned(),
            })
        })
}
