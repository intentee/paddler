use crate::inference_mode_choice::InferenceModeChoice;
use crate::model_preset::ModelPreset;

#[derive(Debug, Clone)]
pub enum StartBalancerFormMessage {
    SetBalancerAddress(String),
    SetInferenceAddress(String),
    SetWebAdminPanelAddress(String),
    SelectInferenceMode(InferenceModeChoice),
    SelectModel(ModelPreset),
    ToggleAddModelLater(bool),
    Confirm,
    Cancel,
}
