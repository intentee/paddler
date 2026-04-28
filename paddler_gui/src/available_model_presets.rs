use crate::model_preset::ModelPreset;
use crate::model_preset_qwen3_0_6b;
use crate::model_preset_qwen3_5_0_8b;

pub fn available_model_presets() -> Vec<ModelPreset> {
    vec![
        model_preset_qwen3_0_6b::preset(),
        model_preset_qwen3_5_0_8b::preset(),
    ]
}
