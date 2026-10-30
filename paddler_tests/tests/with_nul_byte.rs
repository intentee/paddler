#![cfg(feature = "tests_that_use_llms")]

pub fn with_nul_byte(text: &str) -> String {
    format!("{text} \0 byte")
}
