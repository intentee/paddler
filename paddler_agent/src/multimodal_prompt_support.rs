use std::sync::Arc;

use llama_cpp_bindings::mtmd::MtmdContext;

pub struct MultimodalPromptSupport {
    pub multimodal_context: Arc<MtmdContext>,
    pub n_batch: i32,
}
