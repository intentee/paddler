use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::token::LlamaToken;

pub enum ModelBosToken {
    AddedByTokenizer(LlamaToken),
    NotAddedByTokenizer,
}

impl ModelBosToken {
    #[must_use]
    pub fn of(model: &LlamaModel) -> Self {
        match model.token_bos() {
            Some(token) if model.adds_bos_token() => Self::AddedByTokenizer(token),
            Some(_) | None => Self::NotAddedByTokenizer,
        }
    }

    #[must_use]
    pub fn is_duplicated_at_start_of(&self, prompt_tokens: &[LlamaToken]) -> bool {
        match self {
            Self::AddedByTokenizer(token) => prompt_tokens.starts_with(&[*token, *token]),
            Self::NotAddedByTokenizer => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings::token::LlamaToken;

    use super::ModelBosToken;

    const BOS: LlamaToken = LlamaToken(1);
    const TEXT: LlamaToken = LlamaToken(42);

    #[test]
    fn detects_a_rendered_bos_after_the_one_the_tokenizer_added() {
        assert!(ModelBosToken::AddedByTokenizer(BOS).is_duplicated_at_start_of(&[BOS, BOS, TEXT]));
    }

    #[test]
    fn accepts_a_single_leading_bos() {
        assert!(!ModelBosToken::AddedByTokenizer(BOS).is_duplicated_at_start_of(&[BOS, TEXT]));
    }

    #[test]
    fn keeps_repeated_bos_tokens_the_tokenizer_did_not_add() {
        assert!(!ModelBosToken::NotAddedByTokenizer.is_duplicated_at_start_of(&[BOS, BOS, TEXT]));
    }
}
