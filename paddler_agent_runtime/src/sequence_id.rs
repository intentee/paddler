use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::error::memory_seq_rm_error::MemorySeqRmError;
use log::error;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SequenceId(pub u16);

impl SequenceId {
    /// # Errors
    ///
    /// Returns the [`MemorySeqRmError`] llama.cpp reports when it cannot remove the sequence from the KV cache.
    pub fn clear_kv_cache_in(
        self,
        llama_context: &mut LlamaContext<'_>,
    ) -> Result<(), MemorySeqRmError> {
        llama_context.memory_seq_rm(Some(self.kv_cache_sequence_id()), None, None)
    }

    #[must_use]
    pub fn kv_cache_sequence_id(self) -> u32 {
        u32::from(self.0)
    }

    #[must_use]
    pub fn llama_sequence_id(self) -> i32 {
        i32::from(self.0)
    }

    pub fn release_kv_cache_in(
        self,
        llama_context: &mut LlamaContext<'_>,
        agent_name: Option<&str>,
    ) {
        if let Err(clear_error) = self.clear_kv_cache_in(llama_context) {
            error!(
                "{agent_name:?}: {clear_error}; the next request admitted to sequence {self:?} clears it again"
            );
        }
    }
}
