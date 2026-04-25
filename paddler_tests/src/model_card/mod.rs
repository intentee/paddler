use paddler_types::huggingface_model_reference::HuggingFaceModelReference;

pub mod nomic_embed_text_v1_5;
pub mod qwen2_5_vl_3b;
pub mod qwen2_5_vl_3b_mmproj;
pub mod qwen3_0_6b;
pub mod qwen3_5_0_8b;
pub mod qwen3_5_0_8b_mmproj;
pub mod qwen3_embedding_0_6b;
pub mod smolvlm2_256m;
pub mod smolvlm2_256m_mmproj;

pub struct ModelCard {
    pub gpu_layer_count: u32,
    pub reference: HuggingFaceModelReference,
}
