use std::sync::Arc;

use llama_cpp_bindings::mtmd::MtmdBitmap;
use llama_cpp_bindings::mtmd::MtmdContext;

use crate::prepared_multimodal_prompt::PreparedMultimodalPrompt;

pub struct MultimodalPromptSupport {
    pub multimodal_context: Arc<MtmdContext>,
    pub n_batch: i32,
}

impl MultimodalPromptSupport {
    #[must_use]
    pub fn prepare_prompt(
        &self,
        bitmaps: Vec<MtmdBitmap>,
        text: String,
    ) -> PreparedMultimodalPrompt {
        PreparedMultimodalPrompt {
            bitmaps,
            multimodal_context: self.multimodal_context.clone(),
            n_batch: self.n_batch,
            text,
        }
    }
}
