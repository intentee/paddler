use crate::model_preset::ModelPreset;

#[derive(Debug, Clone)]
pub enum StartBalancerFormMessage {
    SetBalancerAddress(String),
    SetInferenceAddress(String),
    SetWebAdminPanelAddress(String),
    SelectModel(ModelPreset),
    ToggleAddModelLater(bool),
    Confirm,
    Cancel,
}
