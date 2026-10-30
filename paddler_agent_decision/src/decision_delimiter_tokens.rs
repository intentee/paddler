use llama_cpp_bindings::llama_token_attr::LlamaTokenAttr;
use llama_cpp_bindings::token::LlamaToken;

use paddler_agent_pointer_head::pointer_head_delimiters::PointerHeadDelimiters;
use paddler_messaging::agent_issue_params::pointer_head_incompatibility::PointerHeadIncompatibility;

use crate::decision_error::DecisionError;
use crate::delimiter_vocabulary::DelimiterVocabulary;

fn resolve_control_token<TVocabulary: DelimiterVocabulary>(
    vocabulary: &TVocabulary,
    delimiter: &str,
) -> Result<LlamaToken, DecisionError> {
    let not_a_control_token = || DecisionError::PointerHeadIncompatibleWithModel {
        incompatibility: PointerHeadIncompatibility::DelimiterIsNotAControlToken {
            delimiter: delimiter.to_owned(),
        },
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
    pub fn resolve<TVocabulary: DelimiterVocabulary>(
        vocabulary: &TVocabulary,
        PointerHeadDelimiters {
            decide,
            option_end,
            option_start,
            question,
            state,
        }: &PointerHeadDelimiters,
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
    use llama_cpp_bindings::error::StringToTokenError;
    use llama_cpp_bindings::error::TokenAttrError;
    use llama_cpp_bindings::llama_token_attrs::LlamaTokenAttrs;
    use llama_cpp_bindings::llama_token_attrs_from_int_error::LlamaTokenAttrsFromIntError;
    use llama_cpp_bindings::token::LlamaToken;

    use paddler_agent_pointer_head::pointer_head_delimiters::PointerHeadDelimiters;

    use super::DecisionDelimiterTokens;
    use crate::decision_error::DecisionError;
    use crate::delimiter_vocabulary::DelimiterVocabulary;

    const UNKNOWN_ATTRIBUTE_BITS: u32 = 1 << 30;

    struct VocabularyWithUnreadableAttributes;

    impl DelimiterVocabulary for VocabularyWithUnreadableAttributes {
        fn tokenize_delimiter(
            &self,
            _delimiter: &str,
        ) -> Result<Vec<LlamaToken>, StringToTokenError> {
            Ok(vec![LlamaToken::new(0)])
        }

        fn token_attributes(&self, _token: LlamaToken) -> Result<LlamaTokenAttrs, TokenAttrError> {
            Err(TokenAttrError::UnknownAttributes(
                LlamaTokenAttrsFromIntError::UnknownValue(UNKNOWN_ATTRIBUTE_BITS),
            ))
        }
    }

    #[test]
    fn a_delimiter_whose_token_attributes_are_unreadable_is_refused() {
        assert_eq!(
            DecisionDelimiterTokens::resolve(
                &VocabularyWithUnreadableAttributes,
                &PointerHeadDelimiters {
                    decide: "<decide>".to_owned(),
                    option_end: "<option_end>".to_owned(),
                    option_start: "<option_start>".to_owned(),
                    question: "<question>".to_owned(),
                    state: "<state>".to_owned(),
                },
            )
            .err()
            .as_ref()
            .map(ToString::to_string),
            Some(
                DecisionError::DelimiterAttributesUnreadable {
                    delimiter: "<decide>".to_owned(),
                    source: TokenAttrError::UnknownAttributes(
                        LlamaTokenAttrsFromIntError::UnknownValue(UNKNOWN_ATTRIBUTE_BITS),
                    ),
                }
                .to_string()
            )
        );
    }
}
