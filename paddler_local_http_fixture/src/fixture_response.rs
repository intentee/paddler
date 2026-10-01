use http::StatusCode;

#[derive(Clone)]
pub enum FixtureResponse {
    CloseBeforeHeaders,
    Ok(Vec<u8>),
    PartialContent {
        body: Vec<u8>,
        content_range: String,
    },
    StallBeforeHeaders,
    StalledBody {
        sent_body: Vec<u8>,
        withheld_byte_count: usize,
    },
    Status(StatusCode),
    TruncatedBody {
        sent_body: Vec<u8>,
        status: StatusCode,
        withheld_byte_count: usize,
    },
}
