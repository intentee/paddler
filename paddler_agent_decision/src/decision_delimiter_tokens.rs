use llama_cpp_model::llama_token::LlamaToken;
use llama_cpp_model::llama_token_attr::LlamaTokenAttr;

use crate::decision_delimiters::DecisionDelimiters;
use crate::decision_error::DecisionError;
use crate::delimiter_vocabulary::DelimiterVocabulary;

fn resolve_control_token<TVocabulary: DelimiterVocabulary>(
    vocabulary: &TVocabulary,
    delimiter: &str,
) -> Result<LlamaToken, DecisionError> {
    let not_a_control_token = || DecisionError::DelimiterIsNotAControlToken {
        delimiter: delimiter.to_owned(),
    };
    let tokens = vocabulary.tokenize_delimiter(delimiter).map_err(|source| {
        DecisionError::DelimiterTokenizationFailed {
            delimiter: delimiter.to_owned(),
            source,
        }
    })?;
    let control_token = match tokens.as_slice() {
        [token] => vocabulary
            .token_attributes(*token)
            .map_err(|source| DecisionError::DelimiterAttributesUnreadable {
                delimiter: delimiter.to_owned(),
                source,
            })?
            .contains(LlamaTokenAttr::Control)
            .then_some(*token),
        _ => None,
    };

    control_token.ok_or_else(not_a_control_token)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecisionDelimiterTokens {
    pub decide: LlamaToken,
    pub option_end: LlamaToken,
    pub option_start: LlamaToken,
    pub question: LlamaToken,
    pub state: LlamaToken,
}

impl DecisionDelimiterTokens {
    /// # Errors
    ///
    /// Returns [`DecisionError::DelimiterTokenizationFailed`] when a delimiter cannot be tokenized, [`DecisionError::DelimiterAttributesUnreadable`] when the attributes of its token cannot be read, and [`DecisionError::DelimiterIsNotAControlToken`] when a delimiter is not a single control token of the vocabulary.
    pub fn resolve<TVocabulary: DelimiterVocabulary>(
        vocabulary: &TVocabulary,
        DecisionDelimiters {
            decide,
            option_end,
            option_start,
            question,
            state,
        }: &DecisionDelimiters<'_>,
    ) -> Result<Self, DecisionError> {
        let mut control_tokens = [LlamaToken::new(0); 5];

        for (control_token, delimiter) in
            control_tokens
                .iter_mut()
                .zip([decide, option_end, option_start, question, state])
        {
            *control_token = resolve_control_token(vocabulary, delimiter)?;
        }

        let [decide, option_end, option_start, question, state] = control_tokens;

        Ok(Self {
            decide,
            option_end,
            option_start,
            question,
            state,
        })
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_model::llama_token::LlamaToken;
    use llama_cpp_model::llama_token_attrs::LlamaTokenAttrs;
    use llama_cpp_model::model_error::ModelError;

    use super::DecisionDelimiterTokens;
    use crate::constants::KEV_DECISION_DELIMITERS;
    use crate::decision_error::DecisionError;
    use crate::delimiter_vocabulary::DelimiterVocabulary;

    const UNKNOWN_ATTRIBUTE_BITS: u32 = 1 << 30;

    struct VocabularyWithUnreadableAttributes;

    impl DelimiterVocabulary for VocabularyWithUnreadableAttributes {
        fn token_attributes(&self, _token: LlamaToken) -> Result<LlamaTokenAttrs, ModelError> {
            Err(ModelError::UnknownTokenAttributes(UNKNOWN_ATTRIBUTE_BITS))
        }

        fn tokenize_delimiter(&self, _delimiter: &str) -> Result<Vec<LlamaToken>, ModelError> {
            Ok(vec![LlamaToken::new(0)])
        }
    }

    #[test]
    fn a_delimiter_whose_token_attributes_are_unreadable_is_refused() {
        assert!(matches!(
            DecisionDelimiterTokens::resolve(
                &VocabularyWithUnreadableAttributes,
                &KEV_DECISION_DELIMITERS
            ),
            Err(DecisionError::DelimiterAttributesUnreadable {
                delimiter,
                source: ModelError::UnknownTokenAttributes(UNKNOWN_ATTRIBUTE_BITS),
            }) if delimiter == KEV_DECISION_DELIMITERS.decide
        ));
    }
}
