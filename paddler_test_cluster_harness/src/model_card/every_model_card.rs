use crate::model_card::ModelCard;
use crate::model_card::deepseek_r1_distill_llama_8b::deepseek_r1_distill_llama_8b;
use crate::model_card::gemma_3_4b_it::gemma_3_4b_it;
use crate::model_card::gemma_3_4b_it_mmproj::gemma_3_4b_it_mmproj;
use crate::model_card::gemma_4_e2b_it::gemma_4_e2b_it;
use crate::model_card::gemma_4_e2b_it_mmproj::gemma_4_e2b_it_mmproj;
use crate::model_card::ministral_3_3b_reasoning::ministral_3_3b_reasoning;
use crate::model_card::ministral_3_8b_reasoning::ministral_3_8b_reasoning;
use crate::model_card::ministral_3_8b_reasoning_mmproj::ministral_3_8b_reasoning_mmproj;
use crate::model_card::nomic_embed_text_v1_5::nomic_embed_text_v1_5;
use crate::model_card::qwen2_5_vl_3b::qwen2_5_vl_3b;
use crate::model_card::qwen2_5_vl_3b_mmproj::qwen2_5_vl_3b_mmproj;
use crate::model_card::qwen3_0_6b::qwen3_0_6b;
use crate::model_card::qwen3_5_0_8b::qwen3_5_0_8b;
use crate::model_card::qwen3_5_0_8b_mmproj::qwen3_5_0_8b_mmproj;
use crate::model_card::smolvlm2_256m::smolvlm2_256m;
use crate::model_card::smolvlm2_256m_mmproj::smolvlm2_256m_mmproj;

#[must_use]
pub fn every_model_card() -> Vec<ModelCard> {
    vec![
        deepseek_r1_distill_llama_8b(),
        gemma_3_4b_it(),
        gemma_3_4b_it_mmproj(),
        gemma_4_e2b_it(),
        gemma_4_e2b_it_mmproj(),
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
    ]
}
