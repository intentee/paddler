use crate::model_preset::ModelPreset;

pub struct StartBalancerConfigData {
    pub add_model_later: bool,
    pub balancer_address: String,
    pub balancer_address_error: Option<String>,
    pub inference_address: String,
    pub inference_address_error: Option<String>,
    pub model_error: Option<String>,
    pub selected_model: Option<ModelPreset>,
    pub starting: bool,
}
