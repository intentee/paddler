use serde::Serialize;

#[derive(Serialize)]
pub struct BufferedRequestManagerSnapshot {
    pub buffered_requests_current: i32,
}
