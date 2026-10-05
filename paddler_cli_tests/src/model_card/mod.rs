pub mod qwen3_0_6b;
pub mod qwen3_embedding_0_6b;

use paddler_messaging::huggingface_model_reference::HuggingFaceModelReference;

pub struct ModelCard {
    pub gpu_layer_count: i32,
    pub reference: HuggingFaceModelReference,
}
