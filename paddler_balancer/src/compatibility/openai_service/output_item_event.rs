use serde::Serialize;

use crate::compatibility::openai_service::responses_output_item::ResponsesOutputItem;

#[derive(Clone, Debug, Serialize)]
pub struct OutputItemEvent {
    pub sequence_number: u64,
    pub output_index: usize,
    pub item: ResponsesOutputItem,
}
