#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SequenceId(pub u16);

impl SequenceId {
    #[must_use]
    pub fn llama_sequence_id(self) -> i32 {
        i32::from(self.0)
    }

    #[must_use]
    pub fn kv_cache_sequence_id(self) -> u32 {
        u32::from(self.0)
    }
}
