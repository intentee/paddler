use serde::Deserialize;

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatCompletionStreamOptions {
    #[serde(default)]
    pub include_usage: bool,
}
