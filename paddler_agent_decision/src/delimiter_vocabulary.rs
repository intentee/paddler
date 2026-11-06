use llama_cpp_bindings::error::string_to_token_error::StringToTokenError;
use llama_cpp_bindings::error::token_attr_error::TokenAttrError;
use llama_cpp_bindings::llama_token_attrs::LlamaTokenAttrs;
use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::model::add_bos::AddBos;
use llama_cpp_bindings::model::parse_special_tokens::ParseSpecialTokens;
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
