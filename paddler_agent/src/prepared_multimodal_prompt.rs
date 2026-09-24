use std::sync::Arc;

use llama_cpp_bindings::mtmd::MtmdBitmap;
use llama_cpp_bindings::mtmd::MtmdContext;

pub struct PreparedMultimodalPrompt {
    pub bitmaps: Vec<MtmdBitmap>,
    pub multimodal_context: Arc<MtmdContext>,
    pub n_batch: i32,
    pub text: String,
}
