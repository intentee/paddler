use std::time::Duration;

use http::StatusCode;

#[derive(Clone)]
pub enum FixtureResponse {
    EndlessTrickle {
        byte_interval: Duration,
    },
    HuggingFaceFile {
        body: Vec<u8>,
        commit_hash: String,
        etag: String,
    },
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
    TruncatedHuggingFaceFile {
        commit_hash: String,
        etag: String,
        sent_body: Vec<u8>,
        withheld_byte_count: usize,
    },
}
