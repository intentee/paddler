use serde::Deserialize;

#[derive(Deserialize)]
pub struct ChatCompletionRequestedFunction {
    pub arguments: String,
    pub name: String,
}
