use std::path::PathBuf;

use paddler_agent::agent_applicable_state::AgentApplicableState;
use paddler_agent::applicable_decisions::ApplicableDecisions;
use paddler_agent::inference_capabilities::InferenceCapabilities;
use paddler_agent::model_serving_settings::ModelServingSettings;

use crate::settings_whose_model_cannot_load::settings_whose_model_cannot_load;

pub fn state_enabling_decisions() -> AgentApplicableState {
    let settings_whose_model_cannot_load = settings_whose_model_cannot_load();

    AgentApplicableState::WithModel(ModelServingSettings {
        capabilities: InferenceCapabilities {
            decisions: ApplicableDecisions::Enabled {
                pointer_head_path: PathBuf::from("/nonexistent/pointer_head.gguf"),
            },
            ..settings_whose_model_cannot_load.capabilities
        },
        ..settings_whose_model_cannot_load
    })
}
