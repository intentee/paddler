use crate::model_preset::ModelPreset;

pub struct StartClusterConfigData {
    pub add_model_later: bool,
    pub cluster_address: String,
    pub cluster_address_error: Option<String>,
    pub inference_address: String,
    pub inference_address_error: Option<String>,
    pub model_error: Option<String>,
    pub selected_model: Option<ModelPreset>,
    pub starting: bool,
    pub web_admin_panel_address: String,
    pub web_admin_panel_address_error: Option<String>,
    pub web_admin_panel_address_placeholder: String,
}
