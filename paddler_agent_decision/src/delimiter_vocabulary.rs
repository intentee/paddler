use llama_cpp_bindings::error::StringToTokenError;
use llama_cpp_bindings::error::TokenAttrError;
use llama_cpp_bindings::llama_token_attrs::LlamaTokenAttrs;
use llama_cpp_bindings::model::AddBos;
use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::model::ParseSpecialTokens;
use llama_cpp_bindings::token::LlamaToken;

pub trait DelimiterVocabulary {
    fn tokenize_delimiter(&self, delimiter: &str) -> Result<Vec<LlamaToken>, StringToTokenError>;

    fn token_attributes(&self, token: LlamaToken) -> Result<LlamaTokenAttrs, TokenAttrError>;
}

impl DelimiterVocabulary for LlamaModel {
    fn tokenize_delimiter(&self, delimiter: &str) -> Result<Vec<LlamaToken>, StringToTokenError> {
        self.str_to_token(delimiter, AddBos::Never, ParseSpecialTokens::Always)
    }

    fn token_attributes(&self, token: LlamaToken) -> Result<LlamaTokenAttrs, TokenAttrError> {
        self.token_attr(token)
    }
}
