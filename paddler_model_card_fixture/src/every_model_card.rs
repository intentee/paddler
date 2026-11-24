use crate::deepseek_r1_distill_llama_8b::deepseek_r1_distill_llama_8b;
use crate::gemma_3_4b_it::gemma_3_4b_it;
use crate::gemma_3_4b_it_mmproj::gemma_3_4b_it_mmproj;
use crate::gemma_4_e2b_it::gemma_4_e2b_it;
use crate::gemma_4_e2b_it_mmproj::gemma_4_e2b_it_mmproj;
use crate::kev_0_8b::kev_0_8b;
use crate::lfm2_colbert_350m::lfm2_colbert_350m;
use crate::ministral_3_3b_reasoning::ministral_3_3b_reasoning;
use crate::ministral_3_8b_reasoning::ministral_3_8b_reasoning;
use crate::ministral_3_8b_reasoning_mmproj::ministral_3_8b_reasoning_mmproj;
use crate::model_card::ModelCard;
use crate::nomic_embed_text_v1_5::nomic_embed_text_v1_5;
use crate::qwen2_5_vl_3b::qwen2_5_vl_3b;
use crate::qwen2_5_vl_3b_mmproj::qwen2_5_vl_3b_mmproj;
use crate::qwen3_0_6b::qwen3_0_6b;
use crate::qwen3_5_0_8b::qwen3_5_0_8b;
use crate::qwen3_5_0_8b_mmproj::qwen3_5_0_8b_mmproj;
use crate::smolvlm2_256m::smolvlm2_256m;
use crate::smolvlm2_256m_mmproj::smolvlm2_256m_mmproj;
use crate::ultravox_v0_5_llama_3_2_1b::ultravox_v0_5_llama_3_2_1b;
use crate::ultravox_v0_5_llama_3_2_1b_mmproj::ultravox_v0_5_llama_3_2_1b_mmproj;

#[must_use]
pub fn every_model_card() -> Vec<ModelCard> {
    vec![
        deepseek_r1_distill_llama_8b(),
        gemma_3_4b_it(),
        gemma_3_4b_it_mmproj(),
        gemma_4_e2b_it(),
        gemma_4_e2b_it_mmproj(),
        kev_0_8b(),
        lfm2_colbert_350m(),
        ministral_3_3b_reasoning(),
        ministral_3_8b_reasoning(),
        ministral_3_8b_reasoning_mmproj(),
        nomic_embed_text_v1_5(),
        qwen2_5_vl_3b(),
        qwen2_5_vl_3b_mmproj(),
        qwen3_0_6b(),
        qwen3_5_0_8b(),
        qwen3_5_0_8b_mmproj(),
        smolvlm2_256m(),
        smolvlm2_256m_mmproj(),
        ultravox_v0_5_llama_3_2_1b(),
        ultravox_v0_5_llama_3_2_1b_mmproj(),
    ]
}
