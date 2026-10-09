use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub enum PointerHeadIncompatibility {
    DelimiterIsNotAControlToken {
        delimiter: String,
    },
    HiddenSizeMismatch {
        model_hidden_size: usize,
        pointer_head_hidden_size: usize,
    },
}
