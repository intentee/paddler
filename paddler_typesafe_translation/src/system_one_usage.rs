use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SystemOneUsage {
    pub input_tokens: usize,
    pub output_tokens: usize,
}
