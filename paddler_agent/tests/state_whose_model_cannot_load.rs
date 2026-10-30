use paddler_agent::agent_applicable_state::AgentApplicableState;

use crate::settings_whose_model_cannot_load::settings_whose_model_cannot_load;

pub fn state_whose_model_cannot_load() -> AgentApplicableState {
    AgentApplicableState::WithModel(settings_whose_model_cannot_load())
}
