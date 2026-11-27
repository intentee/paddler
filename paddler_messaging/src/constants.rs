use std::num::NonZeroU16;
use std::time::Duration;

pub const DECISION_SLOTS_WITH_A_QUESTION_LANE: NonZeroU16 = NonZeroU16::MIN.saturating_add(2 - 1);
pub const PING_INTERVAL: Duration = Duration::from_secs(3);
pub const UNANSWERED_PINGS_TOLERATED: u32 = 1;
