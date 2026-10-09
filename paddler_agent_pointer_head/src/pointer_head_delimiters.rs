use llama_cpp_bindings::gguf_context::GgufContext;

use crate::pointer_head_error::PointerHeadError;

const DELIMITER_KEYS: [&str; 5] = [
    "pointer_head.delimiter.decide",
    "pointer_head.delimiter.option_end",
    "pointer_head.delimiter.option_start",
    "pointer_head.delimiter.question",
    "pointer_head.delimiter.state",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PointerHeadDelimiters {
    pub decide: String,
    pub option_end: String,
    pub option_start: String,
    pub question: String,
    pub state: String,
}

impl PointerHeadDelimiters {
    pub fn load(gguf_context: &GgufContext) -> Result<Self, PointerHeadError> {
        let mut delimiter_tokens: [String; 5] = Default::default();

        for (delimiter_token, key) in delimiter_tokens.iter_mut().zip(DELIMITER_KEYS) {
            gguf_context
                .find_key(key)
                .and_then(|key_id| gguf_context.val_str(key_id))
                .map_err(|source| PointerHeadError::KeyUnreadable {
                    key: key.to_owned(),
                    source,
                })?
                .clone_into(delimiter_token);
        }

        let [decide, option_end, option_start, question, state] = delimiter_tokens;

        Ok(Self {
            decide,
            option_end,
            option_start,
            question,
            state,
        })
    }
}
