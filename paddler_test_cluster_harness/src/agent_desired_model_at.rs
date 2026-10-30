use std::path::Path;

use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_model_source::agent_local_model_path::AgentLocalModelPath;
use paddler_model_source::model_source::ModelSource;
use paddler_model_source::model_source_error::ModelSourceError;

pub fn agent_desired_model_at(path: &Path) -> Result<AgentDesiredModel, ModelSourceError> {
    Ok(
        ModelSource::LocalToAgent(AgentLocalModelPath::from_absolute_path(path)?)
            .into_agent_desired_model(),
    )
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use paddler_model_source::model_source_error::ModelSourceError;

    use super::agent_desired_model_at;

    #[test]
    fn a_model_at_a_relative_path_cannot_be_desired() {
        assert_eq!(
            agent_desired_model_at(Path::new("model.gguf"))
                .unwrap_err()
                .to_string(),
            ModelSourceError::LocalModelPathNotAbsolute {
                path: "model.gguf".into(),
            }
            .to_string()
        );
    }
}
