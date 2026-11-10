use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_model_source::model_source_error::ModelSourceError;

use crate::agent_desired_model_at::agent_desired_model_at;
use crate::fixture_path::fixture_path;

/// # Errors
///
/// Returns [`ModelSourceError::LocalModelPathNotAbsolute`] when the fixture path is not absolute, and [`ModelSourceError::LocalModelPathClimbsToParentDirectory`] when it climbs to a parent directory.
pub fn pointer_head_fixture(
    fixture_file_name: &str,
) -> Result<AgentDesiredModel, ModelSourceError> {
    agent_desired_model_at(&fixture_path(fixture_file_name))
}
