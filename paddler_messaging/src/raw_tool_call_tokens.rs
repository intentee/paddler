use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawToolCallTokens {
    pub text: String,
    pub ffi_error_message: String,
}

#[cfg(test)]
mod tests {
    use super::RawToolCallTokens;

    #[test]
    fn carries_text_and_ffi_error_message() {
        let tokens = RawToolCallTokens {
            text: "raw payload".to_owned(),
            ffi_error_message: "parser bailed".to_owned(),
        };

        assert_eq!(tokens.text, "raw payload");
        assert_eq!(tokens.ffi_error_message, "parser bailed");
    }
}
