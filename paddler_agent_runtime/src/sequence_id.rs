use llama_cpp_context::llama_context::LlamaContext;
use llama_cpp_context_memory::context_memory::ContextMemory as _;
use llama_cpp_context_memory::context_memory_error::ContextMemoryError;
use log::error;

use paddler_error_chain::error_chain::ErrorChain;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SequenceId(pub u16);

impl SequenceId {
    /// # Errors
    ///
    /// Returns the [`ContextMemoryError`] llama.cpp reports when it cannot remove the sequence from the KV cache.
    pub fn clear_kv_cache_in(
        self,
        llama_context: &LlamaContext<'_>,
    ) -> Result<(), ContextMemoryError> {
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

    pub fn release_kv_cache_in(self, llama_context: &LlamaContext<'_>, agent_name: Option<&str>) {
        if let Err(clear_error) = self.clear_kv_cache_in(llama_context) {
            error!(
                "{agent_name:?}: {}; the next request admitted to sequence {self:?} clears it again",
                ErrorChain {
                    error: &clear_error
                }
            );
        }
    }
}
