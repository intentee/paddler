use llama_cpp_model::add_bos::AddBos;
use llama_cpp_model::llama_model::LlamaModel;
use llama_cpp_model::llama_token::LlamaToken;
use llama_cpp_model::llama_token_attrs::LlamaTokenAttrs;
use llama_cpp_model::model_error::ModelError;
use llama_cpp_model::parse_special_tokens::ParseSpecialTokens;

pub trait DelimiterVocabulary {
    /// # Errors
    ///
    /// Returns a [`ModelError`] when the vocabulary cannot report the attributes of the token.
    fn token_attributes(&self, token: LlamaToken) -> Result<LlamaTokenAttrs, ModelError>;

    /// # Errors
    ///
    /// Returns a [`ModelError`] when the vocabulary cannot tokenize the delimiter.
    fn tokenize_delimiter(&self, delimiter: &str) -> Result<Vec<LlamaToken>, ModelError>;
}

impl DelimiterVocabulary for LlamaModel {
    fn token_attributes(&self, token: LlamaToken) -> Result<LlamaTokenAttrs, ModelError> {
        self.token_attr(token)
    }

    fn tokenize_delimiter(&self, delimiter: &str) -> Result<Vec<LlamaToken>, ModelError> {
        self.str_to_token(delimiter, AddBos::Never, ParseSpecialTokens::Always)
    }
}
