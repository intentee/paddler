use std::sync::Arc;

use llama_cpp_model::chat_templates::ChatTemplates;
use llama_cpp_model::llama_chat_template::LlamaChatTemplate;
use llama_cpp_model::llama_model::LlamaModel;
use llama_cpp_model::model_error::ModelError;
use llama_cpp_model_capabilities::conversation_templates::ConversationTemplates;

use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::chat_template_source::ChatTemplateSource;

use crate::agent_model_capabilities_error::AgentModelCapabilitiesError;

pub enum ConversationChatTemplates {
    Absent,
    Present(Arc<ChatTemplates>),
}

impl ConversationChatTemplates {
    /// # Errors
    ///
    /// Returns [`AgentModelCapabilitiesError::ChatTemplateContainsNul`] when the override contains a NUL byte, and [`AgentModelCapabilitiesError::ChatTemplateDoesNotCompile`] when llama.cpp cannot prepare the override or the template embedded in the model.
    pub fn resolve(
        chat_template_source: &ChatTemplateSource,
        model: &LlamaModel,
    ) -> Result<Self, AgentModelCapabilitiesError> {
        match chat_template_source {
            ChatTemplateSource::EmbeddedInModel => match model.chat_templates() {
                Ok(embedded_chat_templates) => Ok(Self::Present(embedded_chat_templates)),
                Err(ModelError::ModelHasNoChatTemplate) => Ok(Self::Absent),
                Err(compile_error) => Err(AgentModelCapabilitiesError::ChatTemplateDoesNotCompile(
                    compile_error,
                )),
            },
            ChatTemplateSource::Override(ChatTemplate { content }) => model
                .chat_templates_with_override(
                    &LlamaChatTemplate::new(content)
                        .map_err(AgentModelCapabilitiesError::ChatTemplateContainsNul)?,
                )
                .map(|override_chat_templates| Self::Present(Arc::new(override_chat_templates)))
                .map_err(AgentModelCapabilitiesError::ChatTemplateDoesNotCompile),
        }
    }

    #[must_use]
    pub fn conversation_templates(&self) -> ConversationTemplates<'_> {
        match self {
            Self::Absent => ConversationTemplates::Absent,
            Self::Present(chat_templates) => ConversationTemplates::Present(chat_templates),
        }
    }
}
