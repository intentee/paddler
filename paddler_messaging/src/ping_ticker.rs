use tokio::time::Interval;
use tokio::time::MissedTickBehavior;
use tokio::time::interval;

use crate::constants::PING_INTERVAL;

#[must_use]
pub fn ping_ticker() -> Interval {
    let mut ping_ticker = interval(PING_INTERVAL);

    ping_ticker.reset();
    ping_ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

    ping_ticker
}
