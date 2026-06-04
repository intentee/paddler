pub mod deepseek_r1_distill_llama_8b;
pub mod gemma_4_e4b_it;
pub mod gemma_4_e4b_it_mmproj;
pub mod glm_4_7_flash;
pub mod ministral_3_14b_reasoning;
pub mod ministral_3_14b_reasoning_mmproj;
pub mod nomic_embed_text_v1_5;
pub mod qwen2_5_vl_3b;
pub mod qwen2_5_vl_3b_mmproj;
pub mod qwen3_0_6b;
pub mod qwen3_5_0_8b;
pub mod qwen3_5_0_8b_mmproj;
pub mod qwen3_6_35b_a3b;
pub mod qwen3_6_35b_a3b_mmproj;
pub mod qwen3_embedding_0_6b;
pub mod smolvlm2_256m;
pub mod smolvlm2_256m_mmproj;

use paddler_messaging::huggingface_model_reference::HuggingFaceModelReference;

pub struct ModelCard {
    pub gpu_layer_count: u32,
    pub reference: HuggingFaceModelReference,
}
