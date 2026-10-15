use serde::Deserialize;

use crate::responses_function_output::ResponsesFunctionOutput;

#[derive(Deserialize)]
pub struct ResponsesFunctionCallOutputItem {
    pub output: ResponsesFunctionOutput,
}
