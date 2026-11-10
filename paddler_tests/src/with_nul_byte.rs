#[must_use]
pub fn with_nul_byte(text: &str) -> String {
    format!("{text} \0 byte")
}
