use serde::Deserialize;

use crate::tokenizer_reference_entry::TokenizerReferenceEntry;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TokenizerReference {
    pub entries: Vec<TokenizerReferenceEntry>,
    pub input_normalization: String,
}
