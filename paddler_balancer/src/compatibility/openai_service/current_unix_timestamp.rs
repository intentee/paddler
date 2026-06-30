use std::time::SystemTime;

use chrono::DateTime;
use chrono::Utc;

#[must_use]
pub fn current_unix_timestamp() -> u64 {
    DateTime::<Utc>::from(SystemTime::now())
        .timestamp()
        .unsigned_abs()
}
