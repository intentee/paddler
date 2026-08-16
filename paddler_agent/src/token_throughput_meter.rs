use std::time::Duration;
use std::time::Instant;

use parking_lot::Mutex;

/// How long a window stays open before its token count is turned into a
/// tokens/sec measurement. One second keeps the reported rate intuitive
/// (it is, literally, "tokens counted in the last second").
const WINDOW_DURATION: Duration = Duration::from_secs(1);

/// A measurement is considered stale, and reported as `0.0`, once this much
/// time has passed without a new token. Without this, a slot that produced a
/// quick burst and then went idle would keep reporting its last burst's rate
/// forever.
const STALE_AFTER: Duration = Duration::from_secs(2);

struct ThroughputWindow {
    tokens_per_second: f64,
    window_started_at: Instant,
    window_token_count: f64,
}

/// Tracks generated tokens for a single agent and derives a continuously
/// updated, approximate tokens-per-second throughput value out of them.
///
/// The meter is intentionally simple: it counts tokens in a rolling
/// one-second window and, once the window closes, turns that count into the
/// reported rate. There is nothing to configure.
pub struct TokenThroughputMeter {
    window: Mutex<ThroughputWindow>,
}

impl TokenThroughputMeter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            window: Mutex::new(ThroughputWindow {
                tokens_per_second: 0.0,
                window_started_at: Instant::now(),
                window_token_count: 0.0,
            }),
        }
    }

    /// Records a single generated token, closing out and measuring the
    /// current window if it has been open for at least [`WINDOW_DURATION`].
    pub fn record_token(&self) {
        let mut window = self.window.lock();
        let elapsed = window.window_started_at.elapsed();

        window.window_token_count += 1.0;

        if elapsed >= WINDOW_DURATION {
            window.tokens_per_second = window.window_token_count / elapsed.as_secs_f64();
            window.window_token_count = 0.0;
            window.window_started_at = Instant::now();
        }
    }

    /// Returns the most recently measured tokens-per-second rate, or `0.0`
    /// if generation has been idle for longer than [`STALE_AFTER`].
    #[must_use]
    pub fn tokens_per_second(&self) -> f64 {
        let window = self.window.lock();

        if window.window_started_at.elapsed() > STALE_AFTER && window.window_token_count == 0.0 {
            return 0.0;
        }

        window.tokens_per_second
    }
}

impl Default for TokenThroughputMeter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::thread::sleep;

    use super::*;

    #[test]
    fn reports_zero_before_any_tokens_are_recorded() {
        let meter = TokenThroughputMeter::new();

        let actual = meter.tokens_per_second();
        assert!((actual - 0.0).abs() < 0.0001);
    }

    #[test]
    fn measures_rate_once_a_window_closes() {
        let meter = TokenThroughputMeter::new();

        for _ in 0..5 {
            meter.record_token();
        }

        sleep(WINDOW_DURATION + Duration::from_millis(50));

        meter.record_token();

        let rate = meter.tokens_per_second();

        assert!(rate > 0.0, "expected a positive rate, got {rate}");
    }

    #[test]
    fn decays_to_zero_after_being_idle() {
        let meter = TokenThroughputMeter::new();

        for _ in 0..5 {
            meter.record_token();
        }

        sleep(WINDOW_DURATION + Duration::from_millis(50));
        meter.record_token();
        sleep(STALE_AFTER + Duration::from_millis(50));

        let actual = meter.tokens_per_second();
        assert!((actual - 0.0).abs() < 0.0001);
    }
}
