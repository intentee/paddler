use serde::Deserialize;

use crate::compatibility::openai_service::openai_responses_function_output::OpenAIResponsesFunctionOutput;

#[derive(Deserialize)]
pub struct OpenAIResponsesFunctionCallOutputItem {
    pub output: OpenAIResponsesFunctionOutput,
}
