use serde::Serialize;

use crate::compatibility::openai_service::responses_content_part::ResponsesContentPart;

#[derive(Clone, Debug, Serialize)]
pub struct ContentPartEvent {
    pub sequence_number: u64,
    pub item_id: String,
    pub output_index: usize,
    pub content_index: usize,
    pub part: ResponsesContentPart,
}
