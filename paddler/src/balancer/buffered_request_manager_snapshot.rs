use serde::Deserialize;
use serde::Serialize;

#[derive(Deserialize, Serialize)]
pub struct BufferedRequestManagerSnapshot {
    pub buffered_requests_current: i32,
}
