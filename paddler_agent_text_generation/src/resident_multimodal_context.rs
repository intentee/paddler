use std::ops::Deref;
use std::path::Path;

use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::mtmd::MtmdContext;
use llama_cpp_bindings::mtmd::MtmdContextParams;
use llama_cpp_bindings::mtmd::MtmdInitError;

use paddler_agent_runtime::vram_occupancy::VramOccupancy;

#[derive(Debug)]
pub struct ResidentMultimodalContext {
    pub multimodal_context: MtmdContext,
    pub vram_occupancy: VramOccupancy,
}

impl ResidentMultimodalContext {
    pub fn load(
        multimodal_projection_path: &Path,
        llama_model: &LlamaModel,
        vram_occupancy: VramOccupancy,
    ) -> Result<Self, MtmdInitError> {
        #[expect(
            clippy::disallowed_methods,
            reason = "the multimodal projection is loaded here, paired with its VRAM occupancy"
        )]
        let initialized_multimodal_context = MtmdContext::init_from_file(
            &multimodal_projection_path.to_string_lossy(),
            llama_model,
            &MtmdContextParams::default(),
        );

        Ok(Self {
            multimodal_context: initialized_multimodal_context?,
            vram_occupancy,
        })
    }
}

impl Deref for ResidentMultimodalContext {
    type Target = MtmdContext;

    fn deref(&self) -> &MtmdContext {
        &self.multimodal_context
    }
}
