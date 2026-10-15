use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_model_source::model_source::ModelSource;

use crate::fixture_path::fixture_path;

#[must_use]
pub fn pointer_head_fixture(fixture_file_name: &str) -> AgentDesiredModel {
    ModelSource::LocalToAgent(fixture_path(fixture_file_name).display().to_string())
        .into_agent_desired_model()
}
