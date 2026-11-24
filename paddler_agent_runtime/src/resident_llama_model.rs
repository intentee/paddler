use std::ops::Deref;
use std::sync::Arc;

use llama_cpp_backend::llama_backend::LlamaBackend;
use llama_cpp_model::llama_model::LlamaModel;

use crate::vram_occupancy::VramOccupancy;

pub struct ResidentLlamaModel {
    pub llama_model: Arc<LlamaModel>,
    pub llama_backend: Arc<LlamaBackend>,
    pub vram_occupancy: VramOccupancy,
}

impl Deref for ResidentLlamaModel {
    type Target = LlamaModel;

    fn deref(&self) -> &LlamaModel {
        &self.llama_model
    }
}
