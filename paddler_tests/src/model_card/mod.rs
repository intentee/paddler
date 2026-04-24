use paddler_types::huggingface_model_reference::HuggingFaceModelReference;

pub mod qwen3_0_6b;
pub mod qwen3_embedding_0_6b;

pub struct ModelCard {
    pub gpu_layer_count: u32,
    pub reference: HuggingFaceModelReference,
}
