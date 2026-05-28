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
