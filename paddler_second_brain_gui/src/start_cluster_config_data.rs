use crate::model_preset::ModelPreset;

#[derive(Default)]
pub struct StartClusterConfigData {
    pub error: Option<String>,
    pub selected_model: Option<ModelPreset>,
    pub run_agent_locally: bool,
    pub starting: bool,
}
