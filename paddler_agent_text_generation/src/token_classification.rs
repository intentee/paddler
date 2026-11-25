use llama_cpp_bindings::llama_cpp_error::LlamaCppError;
use llama_cpp_bindings::sampled_token_classifier::SampledTokenClassifier;
use llama_cpp_chat::chat_session::ChatSession;
use llama_cpp_model::llama_model::LlamaModel;

pub enum TokenClassification {
    ChatSession(ChatSession),
    Undeterminable,
}

impl TokenClassification {
    /// # Errors
    ///
    /// Returns the [`LlamaCppError`] llama.cpp reports when the answer classifier of the chat session cannot read the empty answer.
    pub fn into_classifier(
        self,
        model: &LlamaModel,
    ) -> Result<SampledTokenClassifier<'_>, LlamaCppError> {
        match self {
            Self::ChatSession(session) => SampledTokenClassifier::for_chat_session(model, session),
            Self::Undeterminable => Ok(SampledTokenClassifier::undeterminable(model)),
        }
    }
}
