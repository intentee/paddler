use serde::Deserialize;
use serde::Serialize;

use crate::grammar_constraint::GrammarConstraint;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContinueFromRawPromptParams {
    #[serde(default)]
    pub grammar: Option<GrammarConstraint>,
    pub max_tokens: i32,
    pub raw_prompt: String,
}

#[cfg(test)]
mod tests {
    use serde_json::from_value;
    use serde_json::json;

    use super::ContinueFromRawPromptParams;

    #[test]
    fn a_request_that_omits_the_grammar_field_keeps_working() {
        let request_without_grammar = json!({
            "max_tokens": 10,
            "raw_prompt": "Hello",
        });

        let params: ContinueFromRawPromptParams = from_value(request_without_grammar)
            .expect("a request that omits the grammar field must deserialize");

        assert_eq!(params.grammar, None);
    }
}
