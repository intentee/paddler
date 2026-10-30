use std::collections::BTreeSet;

use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;

use crate::conversation_prompt_support::ConversationPromptSupport;
use crate::text_generation_error::TextGenerationError;

pub enum ConversationSupport {
    Available(Box<ConversationPromptSupport>),
    ChatTemplateMissing(ModelPath),
}

impl ConversationSupport {
    pub fn prompt_support(&self) -> Result<&ConversationPromptSupport, TextGenerationError> {
        match self {
            Self::Available(conversation_prompt_support) => Ok(conversation_prompt_support),
            Self::ChatTemplateMissing(model_path) => {
                Err(TextGenerationError::ConversationRequiresChatTemplate {
                    model_path: model_path.clone(),
                })
            }
        }
    }

    #[must_use]
    pub fn serving_issues(&self) -> BTreeSet<AgentIssue> {
        match self {
            Self::Available(_) => BTreeSet::new(),
            Self::ChatTemplateMissing(model_path) => {
                BTreeSet::from([AgentIssue::UnableToFindChatTemplate(model_path.clone())])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use paddler_messaging::agent_issue::AgentIssue;
    use paddler_messaging::agent_issue_params::model_path::ModelPath;

    use super::ConversationSupport;

    fn model_path() -> ModelPath {
        ModelPath {
            model_path: "nomic.gguf".to_owned(),
        }
    }

    #[test]
    fn a_model_without_a_chat_template_refuses_conversation_prompts() {
        assert_eq!(
            ConversationSupport::ChatTemplateMissing(model_path())
                .prompt_support()
                .err()
                .as_ref()
                .map(ToString::to_string),
            Some(
                "the model nomic.gguf has no chat template, so it cannot continue a conversation"
                    .to_owned()
            )
        );
    }

    #[test]
    fn a_model_without_a_chat_template_reports_that_it_cannot_find_one() {
        assert_eq!(
            ConversationSupport::ChatTemplateMissing(model_path()).serving_issues(),
            BTreeSet::from([AgentIssue::UnableToFindChatTemplate(model_path())])
        );
    }
}
