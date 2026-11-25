use std::time::Duration;

#[must_use]
pub const fn short_buffered_request_timeout() -> Duration {
    Duration::from_millis(50)
}
