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
    use std::error::Error as _;
    use std::time::Duration;
    use std::time::SystemTimeError;
    use std::time::UNIX_EPOCH;

    use super::timestamp_from;
    use crate::openai_translation_error::OpenAITranslationError;

    #[test]
    fn counts_whole_seconds_since_the_unix_epoch() {
        assert_eq!(
            timestamp_from(UNIX_EPOCH + Duration::from_millis(42_500)).ok(),
            Some(42)
        );
    }

    #[test]
    fn refuses_a_time_before_the_unix_epoch() {
        let translation_error = timestamp_from(UNIX_EPOCH - Duration::from_secs(1))
            .expect_err("a time before the Unix epoch must be refused");

        assert_eq!(
            translation_error.to_string(),
            OpenAITranslationError::ClockBeforeUnixEpoch(
                UNIX_EPOCH
                    .duration_since(UNIX_EPOCH + Duration::from_secs(1))
                    .expect_err("the Unix epoch must precede the second after it")
            )
            .to_string()
        );
        assert_eq!(
            translation_error
                .source()
                .and_then(|source| source.downcast_ref::<SystemTimeError>())
                .map(SystemTimeError::duration),
            Some(Duration::from_secs(1))
        );
    }
}
