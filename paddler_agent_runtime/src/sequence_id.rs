use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::error::ClearKvCacheSeqError;
use log::error;

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

    pub fn clear_kv_cache_in(
        self,
        llama_context: &mut LlamaContext<'_>,
    ) -> Result<(), ClearKvCacheSeqError> {
        llama_context.clear_kv_cache_seq(Some(self.kv_cache_sequence_id()), None, None)
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
