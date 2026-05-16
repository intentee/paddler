use crate::address_field::AddressField;
use crate::model_preset::ModelPreset;

pub struct StartBalancerFormData {
    pub add_model_later: bool,
    pub balancer_address: AddressField,
    pub inference_address: AddressField,
    pub model_error: Option<String>,
    pub selected_model: Option<ModelPreset>,
    pub starting: bool,
    pub web_admin_panel_address: AddressField,
    pub web_admin_panel_address_placeholder: String,
}
