use crate::model_preset::ModelPreset;

pub struct StartClusterConfigData {
    pub cluster_address: String,
    pub cluster_address_error: Option<String>,
    pub inference_address: String,
    pub inference_address_error: Option<String>,
    pub model_error: Option<String>,
    pub selected_model: Option<ModelPreset>,
    pub starting: bool,
}
