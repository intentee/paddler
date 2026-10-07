use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use crate::openai_translation_error::OpenAITranslationError;

pub fn timestamp_from(now: SystemTime) -> Result<u64, OpenAITranslationError> {
    now.duration_since(UNIX_EPOCH)
        .map(|since_unix_epoch| since_unix_epoch.as_secs())
        .map_err(OpenAITranslationError::ClockBeforeUnixEpoch)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use std::time::UNIX_EPOCH;

    use super::timestamp_from;
    use crate::openai_translation_error::OpenAITranslationError;

    #[test]
    fn counts_whole_seconds_since_the_unix_epoch() {
        assert!(matches!(
            timestamp_from(UNIX_EPOCH + Duration::from_millis(42_500)),
            Ok(seconds) if seconds == 42
        ));
    }

    #[test]
    fn refuses_a_time_before_the_unix_epoch() {
        assert!(matches!(
            timestamp_from(UNIX_EPOCH - Duration::from_secs(1)),
            Err(OpenAITranslationError::ClockBeforeUnixEpoch(clock_error))
                if clock_error.duration() == Duration::from_secs(1)
        ));
    }
}
