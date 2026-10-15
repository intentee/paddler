use std::sync::Arc;

use llama_cpp_bindings::LlamaCppError;
use llama_cpp_bindings::llama_backend::LlamaBackend;
use parking_lot::Mutex;

use crate::llama_backend_state::LlamaBackendState;

pub struct SharedLlamaBackend {
    llama_backend_state: Mutex<LlamaBackendState>,
}

impl SharedLlamaBackend {
    pub fn get_or_init(&self) -> Result<Arc<LlamaBackend>, LlamaCppError> {
        let mut llama_backend_state = self.llama_backend_state.lock();

        match &*llama_backend_state {
            LlamaBackendState::Initialized(llama_backend) => Ok(llama_backend.clone()),
            LlamaBackendState::Uninitialized => {
                let llama_backend = Arc::new(LlamaBackend::init()?);

                *llama_backend_state = LlamaBackendState::Initialized(llama_backend.clone());

                Ok(llama_backend)
            }
        }
    }
}

impl Default for SharedLlamaBackend {
    fn default() -> Self {
        Self {
            llama_backend_state: Mutex::new(LlamaBackendState::Uninitialized),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::SharedLlamaBackend;

    #[test]
    fn every_model_of_an_agent_shares_one_backend() {
        let shared_llama_backend = SharedLlamaBackend::default();

        assert!(Arc::ptr_eq(
            &shared_llama_backend.get_or_init().unwrap(),
            &shared_llama_backend.get_or_init().unwrap()
        ));
    }
}
