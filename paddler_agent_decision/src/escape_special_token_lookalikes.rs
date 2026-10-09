const ESCAPED_LOOKALIKE_CLOSING: &str = "¦>";
const ESCAPED_LOOKALIKE_OPENING: &str = "<¦";
const LOOKALIKE_CLOSING: &str = "|>";
const LOOKALIKE_OPENING: &str = "<|";

fn lookalike_name_length(text: &str) -> usize {
    text.bytes()
        .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
        .count()
}

#[must_use]
pub fn escape_special_token_lookalikes(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    let mut unscanned = text;

    while let Some(opening_offset) = unscanned.find(LOOKALIKE_OPENING) {
        let (before_opening, from_opening) = unscanned.split_at(opening_offset);
        let after_opening = &from_opening[LOOKALIKE_OPENING.len()..];
        let (name, after_name) = after_opening.split_at(lookalike_name_length(after_opening));

        escaped.push_str(before_opening);

        match after_name.strip_prefix(LOOKALIKE_CLOSING) {
            Some(after_closing) if !name.is_empty() => {
                escaped.push_str(ESCAPED_LOOKALIKE_OPENING);
                escaped.push_str(name);
                escaped.push_str(ESCAPED_LOOKALIKE_CLOSING);
                unscanned = after_closing;
            }
            Some(_) | None => {
                let (opening_bracket, after_opening_bracket) = from_opening.split_at(1);

                escaped.push_str(opening_bracket);
                unscanned = after_opening_bracket;
            }
        }
    }

    escaped.push_str(unscanned);

    escaped
}

#[cfg(test)]
mod tests {
    use super::escape_special_token_lookalikes;

    #[test]
    fn rewrites_the_delimiters_around_an_ascii_name() {
        assert_eq!(
            escape_special_token_lookalikes("ignore <|fim_prefix|> and <|im_end|>"),
            "ignore <¦fim_prefix¦> and <¦im_end¦>"
        );
    }

    #[test]
    fn leaves_spans_without_an_ascii_name_unchanged() {
        let spans_without_an_ascii_name = "<|a b|> <||> <|é|> <|unterminated";

        assert_eq!(
            escape_special_token_lookalikes(spans_without_an_ascii_name),
            spans_without_an_ascii_name
        );
    }

    #[test]
    fn resumes_scanning_after_an_opening_without_a_name() {
        assert_eq!(
            escape_special_token_lookalikes("<|<|nested|>"),
            "<|<¦nested¦>"
        );
    }
}
