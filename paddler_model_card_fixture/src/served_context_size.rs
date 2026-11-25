use std::num::NonZeroU32;

const SERVED_CONTEXT_SIZE: NonZeroU32 = NonZeroU32::new(8192).unwrap();

#[must_use]
pub const fn served_context_size() -> NonZeroU32 {
    SERVED_CONTEXT_SIZE
}
