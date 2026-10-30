use std::ops::Deref;
use std::ops::DerefMut;

use llama_cpp_bindings::context::LlamaContext;

use crate::vram_occupancy::VramOccupancy;

pub struct ResidentLlamaContext<'model> {
    pub llama_context: LlamaContext<'model>,
    pub vram_occupancy: VramOccupancy,
}

impl<'model> Deref for ResidentLlamaContext<'model> {
    type Target = LlamaContext<'model>;

    fn deref(&self) -> &LlamaContext<'model> {
        &self.llama_context
    }
}

impl<'model> DerefMut for ResidentLlamaContext<'model> {
    fn deref_mut(&mut self) -> &mut LlamaContext<'model> {
        &mut self.llama_context
    }
}
