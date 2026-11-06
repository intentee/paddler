use std::path::Path;

use llama_cpp_bindings::error::chat_templates_error::ChatTemplatesError;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::chat_template_does_not_compile_params::ChatTemplateDoesNotCompileParams;
use paddler_messaging::agent_issue_params::model_path::ModelPath;

use crate::uncompilable_chat_template::UNCOMPILABLE_CHAT_TEMPLATE_CONTENT;
use crate::uncompilable_chat_template::UNCOMPILABLE_CHAT_TEMPLATE_PARSE_FAILURE;

#[must_use]
pub fn uncompilable_chat_template_issue(model_path: &Path) -> AgentIssue {
    AgentIssue::ChatTemplateDoesNotCompile(ChatTemplateDoesNotCompileParams {
        error: TextGenerationError::ChatTemplateDoesNotCompile(ChatTemplatesError::Reported {
            message: UNCOMPILABLE_CHAT_TEMPLATE_PARSE_FAILURE.to_owned(),
        })
        .to_string(),
        model_path: ModelPath::from(model_path),
        template_content: UNCOMPILABLE_CHAT_TEMPLATE_CONTENT.to_owned(),
    })
}
