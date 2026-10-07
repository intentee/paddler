use llama_cpp_bindings::llama_token_attr::LlamaTokenAttr;
use llama_cpp_bindings::model::AddBos;
use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::model::ParseSpecialTokens;
use llama_cpp_bindings::token::LlamaToken;

use paddler_agent_pointer_head::pointer_head_delimiters::PointerHeadDelimiters;
use paddler_messaging::agent_issue_params::pointer_head_incompatibility::PointerHeadIncompatibility;

use crate::decision_error::DecisionError;

fn resolve_control_token(model: &LlamaModel, delimiter: &str) -> Result<LlamaToken, DecisionError> {
    let not_a_control_token = || DecisionError::PointerHeadIncompatibleWithModel {
        incompatibility: PointerHeadIncompatibility::DelimiterIsNotAControlToken {
            delimiter: delimiter.to_owned(),
        },
    };
    let tokens = model
        .str_to_token(delimiter, AddBos::Never, ParseSpecialTokens::Always)
        .map_err(|source| DecisionError::DelimiterTokenizationFailed {
            delimiter: delimiter.to_owned(),
            source,
        })?;
    let control_token = match tokens.as_slice() {
        [token] => model
            .token_attr(*token)
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
    pub fn resolve(
        model: &LlamaModel,
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
            *control_token = resolve_control_token(model, delimiter)?;
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
