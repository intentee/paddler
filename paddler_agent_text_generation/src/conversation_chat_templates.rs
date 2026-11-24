use llama_cpp_model::chat_templates::ChatTemplates;
use llama_cpp_model::llama_model::LlamaModel;
use llama_cpp_model::model_error::ModelError;

pub enum ConversationChatTemplates {
    EmbeddedInModel,
    Override(ChatTemplates),
}

impl ConversationChatTemplates {
    /// # Errors
    ///
    /// Returns the [`ModelError`] llama.cpp reports when it cannot prepare the chat templates embedded in the model.
    pub fn of_model<'model>(
        &'model self,
        model: &'model LlamaModel,
    ) -> Result<&'model ChatTemplates, ModelError> {
        match self {
            Self::EmbeddedInModel => model.chat_templates(),
            Self::Override(chat_templates) => Ok(chat_templates),
        }
    }
}
