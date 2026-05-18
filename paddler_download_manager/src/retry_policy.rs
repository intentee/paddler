use std::time::Duration;

const DEFAULT_MAX_ATTEMPTS: u32 = 5;
const DEFAULT_INITIAL_BACKOFF: Duration = Duration::from_secs(1);
const DEFAULT_MAX_BACKOFF: Duration = Duration::from_secs(30);

#[derive(Clone, Debug)]
pub struct RetryPolicy {
    pub initial_backoff: Duration,
    pub max_attempts: u32,
    pub max_backoff: Duration,
}

impl RetryPolicy {
    #[must_use]
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let shift = attempt.min(31);
        let scaled = self.initial_backoff.saturating_mul(1_u32 << shift);

        scaled.min(self.max_backoff)
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            initial_backoff: DEFAULT_INITIAL_BACKOFF,
            max_attempts: DEFAULT_MAX_ATTEMPTS,
            max_backoff: DEFAULT_MAX_BACKOFF,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::retry_policy::DEFAULT_INITIAL_BACKOFF;
    use crate::retry_policy::DEFAULT_MAX_ATTEMPTS;
    use crate::retry_policy::DEFAULT_MAX_BACKOFF;
    use crate::retry_policy::RetryPolicy;

    #[test]
    fn delay_for_attempt_zero_returns_initial_backoff() {
        let policy = RetryPolicy::default();

        assert_eq!(policy.delay_for_attempt(0), DEFAULT_INITIAL_BACKOFF);
    }

    #[test]
    fn delay_for_attempt_grows_exponentially_until_max() {
        let policy = RetryPolicy {
            initial_backoff: Duration::from_secs(1),
            max_attempts: 10,
            max_backoff: Duration::from_secs(64),
        };

        assert_eq!(policy.delay_for_attempt(0), Duration::from_secs(1));
        assert_eq!(policy.delay_for_attempt(1), Duration::from_secs(2));
        assert_eq!(policy.delay_for_attempt(2), Duration::from_secs(4));
        assert_eq!(policy.delay_for_attempt(3), Duration::from_secs(8));
        assert_eq!(policy.delay_for_attempt(4), Duration::from_secs(16));
        assert_eq!(policy.delay_for_attempt(5), Duration::from_secs(32));
        assert_eq!(policy.delay_for_attempt(6), Duration::from_secs(64));
    }

    #[test]
    fn delay_for_attempt_caps_at_max_backoff_for_large_attempts() {
        let policy = RetryPolicy::default();

        assert_eq!(policy.delay_for_attempt(100), DEFAULT_MAX_BACKOFF);
        assert_eq!(policy.delay_for_attempt(u32::MAX), DEFAULT_MAX_BACKOFF);
    }

    #[test]
    fn default_policy_matches_documented_values() {
        let policy = RetryPolicy::default();

        assert_eq!(policy.max_attempts, DEFAULT_MAX_ATTEMPTS);
        assert_eq!(policy.initial_backoff, DEFAULT_INITIAL_BACKOFF);
        assert_eq!(policy.max_backoff, DEFAULT_MAX_BACKOFF);
    }
}
