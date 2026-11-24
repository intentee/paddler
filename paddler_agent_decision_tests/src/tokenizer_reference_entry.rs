use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TokenizerReferenceEntry {
    pub text: String,
    pub token_ids: Vec<i32>,
}
