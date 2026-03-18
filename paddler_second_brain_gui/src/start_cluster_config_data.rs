use crate::model_preset::ModelPreset;

pub struct StartClusterConfigData {
    pub bind_address: String,
    pub bind_port: String,
    pub error: Option<String>,
    pub selected_model: Option<ModelPreset>,
    pub starting: bool,
}
