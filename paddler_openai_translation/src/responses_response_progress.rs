use crate::responses_output_item::ResponsesOutputItem;
use crate::responses_usage::ResponsesUsage;

#[derive(Clone, Debug)]
pub enum ResponsesResponseProgress {
    Completed {
        output: Vec<ResponsesOutputItem>,
        usage: ResponsesUsage,
    },
    Failed {
        error_message: String,
    },
    Incomplete {
        output: Vec<ResponsesOutputItem>,
        usage: ResponsesUsage,
    },
    InProgress,
}
