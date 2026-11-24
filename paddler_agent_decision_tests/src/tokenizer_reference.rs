use serde::Deserialize;

use crate::tokenizer_reference_entry::TokenizerReferenceEntry;
use crate::tokenizer_reference_input_normalization::TokenizerReferenceInputNormalization;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TokenizerReference {
    pub entries: Vec<TokenizerReferenceEntry>,
    pub input_normalization: TokenizerReferenceInputNormalization,
}
