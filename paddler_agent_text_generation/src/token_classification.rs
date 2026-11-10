use llama_cpp_bindings::chat_session::ChatSession;
use llama_cpp_bindings::error::parse_chat_message_error::ParseChatMessageError;
use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::sampled_token_classifier::SampledTokenClassifier;

pub enum TokenClassification {
    ChatSession(ChatSession),
    Undeterminable,
}

impl TokenClassification {
    /// # Errors
    ///
    /// Returns the [`ParseChatMessageError`] of the chat session when its answer classifier cannot be created.
    pub fn into_classifier(
        self,
        model: &LlamaModel,
    ) -> Result<SampledTokenClassifier<'_>, ParseChatMessageError> {
        match self {
            Self::ChatSession(session) => SampledTokenClassifier::for_chat_session(model, session),
            Self::Undeterminable => Ok(SampledTokenClassifier::undeterminable(model)),
        }
    }
}
