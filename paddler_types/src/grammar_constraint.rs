use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(tag = "type")]
pub enum GrammarConstraint {
    Gbnf { grammar: String, root: String },
    JsonSchema { schema: String },
}
