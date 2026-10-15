use std::num::NonZeroU32;

#[must_use]
pub fn max_tokens_for_forced_text(forced_text: &str) -> NonZeroU32 {
    forced_text
        .bytes()
        .skip(1)
        .fold(NonZeroU32::MIN, |max_tokens, _forced_byte| {
            max_tokens.saturating_add(1)
        })
}
