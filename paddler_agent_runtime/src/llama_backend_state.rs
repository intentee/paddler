use std::sync::Arc;

use llama_cpp_backend::llama_backend::LlamaBackend;

pub enum LlamaBackendState {
    Initialized(Arc<LlamaBackend>),
    Uninitialized,
}
