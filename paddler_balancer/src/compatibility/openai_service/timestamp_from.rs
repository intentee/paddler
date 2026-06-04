use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use anyhow::Context as _;
use anyhow::Result;

pub fn timestamp_from(now: SystemTime) -> Result<u64> {
    Ok(now
        .duration_since(UNIX_EPOCH)
        .context("system time is before the Unix epoch")?
        .as_secs())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use std::time::SystemTime;
    use std::time::UNIX_EPOCH;

    use super::timestamp_from;

    #[test]
    fn returns_seconds_since_epoch() {
        let timestamp = timestamp_from(SystemTime::now()).unwrap();

        assert!(timestamp > 0);
    }

    #[test]
    fn errors_before_the_unix_epoch() {
        let before_epoch = UNIX_EPOCH - Duration::from_secs(1);

        assert!(timestamp_from(before_epoch).is_err());
    }
}
