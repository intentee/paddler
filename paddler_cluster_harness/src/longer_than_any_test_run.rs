use std::time::Duration;

#[must_use]
pub const fn longer_than_any_test_run() -> Duration {
    Duration::from_hours(1)
}
