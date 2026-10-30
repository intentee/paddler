use serde::Deserialize;

use crate::responses_function_output::ResponsesFunctionOutput;

#[derive(Deserialize)]
pub struct ResponsesFunctionCallOutputItem {
    pub call_id: String,
    pub output: ResponsesFunctionOutput,
}
