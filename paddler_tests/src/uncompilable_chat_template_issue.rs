use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::chat_template_does_not_compile_params::ChatTemplateDoesNotCompileParams;
use paddler_messaging::agent_issue_params::model_path::ModelPath;

use crate::constants::UNCOMPILABLE_CHAT_TEMPLATE_CONTENT;

pub struct UncompilableChatTemplateIssue {
    pub model_path: ModelPath,
}

impl UncompilableChatTemplateIssue {
    #[must_use]
    pub fn is_reported_by(&self, issue: &AgentIssue) -> bool {
        matches!(
            issue,
            AgentIssue::ChatTemplateDoesNotCompile(ChatTemplateDoesNotCompileParams {
                model_path,
                template_content,
                ..
            }) if *model_path == self.model_path
                && template_content == UNCOMPILABLE_CHAT_TEMPLATE_CONTENT
        )
    }
}
