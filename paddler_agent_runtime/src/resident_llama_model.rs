use std::ops::Deref;
use std::sync::Arc;

use llama_cpp_bindings::llama_backend::LlamaBackend;
use llama_cpp_bindings::model::LlamaModel;

use crate::vram_occupancy::VramOccupancy;

pub struct ResidentLlamaModel {
    pub llama_model: LlamaModel,
    pub llama_backend: Arc<LlamaBackend>,
    pub vram_occupancy: VramOccupancy,
}

impl Deref for ResidentLlamaModel {
    type Target = LlamaModel;

    fn deref(&self) -> &LlamaModel {
        &self.llama_model
    }
}
