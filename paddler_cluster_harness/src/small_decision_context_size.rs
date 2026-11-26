use std::num::NonZeroU32;

const SMALL_DECISION_CONTEXT_SIZE: NonZeroU32 = NonZeroU32::MIN.saturating_add(256 - 1);

#[must_use]
pub const fn small_decision_context_size() -> NonZeroU32 {
    SMALL_DECISION_CONTEXT_SIZE
}
