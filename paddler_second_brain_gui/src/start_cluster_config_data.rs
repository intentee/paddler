use crate::model_preset::ModelPreset;

pub struct StartClusterConfigData {
    pub balancer_address: String,
    pub error: Option<String>,
    pub inference_address: String,
    pub selected_model: Option<ModelPreset>,
    pub starting: bool,
}
