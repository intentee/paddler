use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::token::LlamaToken;

pub struct ModelBosToken {
    pub added_by_tokenizer: bool,
    pub token: LlamaToken,
}

impl ModelBosToken {
    #[must_use]
    pub fn of(model: &LlamaModel) -> Self {
        Self {
            added_by_tokenizer: model.adds_bos_token(),
            token: model.token_bos(),
        }
    }

    #[must_use]
    pub fn is_duplicated_at_start_of(&self, prompt_tokens: &[LlamaToken]) -> bool {
        self.added_by_tokenizer && prompt_tokens.starts_with(&[self.token, self.token])
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings::token::LlamaToken;

    use super::ModelBosToken;

    const BOS: LlamaToken = LlamaToken(1);
    const TEXT: LlamaToken = LlamaToken(42);

    fn bos_added_by_tokenizer(added_by_tokenizer: bool) -> ModelBosToken {
        ModelBosToken {
            added_by_tokenizer,
            token: BOS,
        }
    }

    #[test]
    fn detects_a_rendered_bos_after_the_one_the_tokenizer_added() {
        assert!(bos_added_by_tokenizer(true).is_duplicated_at_start_of(&[BOS, BOS, TEXT]));
    }

    #[test]
    fn accepts_a_single_leading_bos() {
        assert!(!bos_added_by_tokenizer(true).is_duplicated_at_start_of(&[BOS, TEXT]));
    }

    #[test]
    fn keeps_repeated_bos_tokens_the_tokenizer_did_not_add() {
        assert!(!bos_added_by_tokenizer(false).is_duplicated_at_start_of(&[BOS, BOS, TEXT]));
    }
}
