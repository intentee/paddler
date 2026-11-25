use std::num::NonZeroU32;

const SMALL_DECISION_CONTEXT_SIZE: NonZeroU32 = NonZeroU32::new(256).unwrap();

#[must_use]
pub const fn small_decision_context_size() -> NonZeroU32 {
    SMALL_DECISION_CONTEXT_SIZE
}
