use crate::responses_response_header::ResponsesResponseHeader;
use crate::responses_stream::ResponsesStream;

pub enum ResponsesDelivery {
    Buffered(ResponsesResponseHeader),
    Streamed(ResponsesStream),
}
