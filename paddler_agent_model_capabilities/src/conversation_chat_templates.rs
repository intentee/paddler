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
    EmbeddedInModel,
    Override(ChatTemplates),
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
                Ok(_embedded_chat_templates) => Ok(Self::EmbeddedInModel),
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
                .map(Self::Override)
                .map_err(AgentModelCapabilitiesError::ChatTemplateDoesNotCompile),
        }
    }

    /// # Errors
    ///
    /// Returns the [`ModelError`] llama.cpp reports when it cannot prepare the chat templates embedded in the model.
    pub fn of_model<'model>(
        &'model self,
        model: &'model LlamaModel,
    ) -> Result<ConversationTemplates<'model>, ModelError> {
        match self {
            Self::Absent => Ok(ConversationTemplates::Absent),
            Self::EmbeddedInModel => model.chat_templates().map(ConversationTemplates::Present),
            Self::Override(chat_templates) => Ok(ConversationTemplates::Present(chat_templates)),
        }
    }
}
