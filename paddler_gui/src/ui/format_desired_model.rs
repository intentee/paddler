use paddler_types::agent_desired_model::AgentDesiredModel;

#[must_use]
pub fn format_desired_model(desired_model: &AgentDesiredModel) -> String {
    match desired_model {
        AgentDesiredModel::HuggingFace(reference) => {
            format!(
                "HuggingFace {}/{} ({})",
                reference.repo_id, reference.filename, reference.revision,
            )
        }
        AgentDesiredModel::LocalToAgent(path) => format!("Local: {path}"),
        AgentDesiredModel::None => "(not set)".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unnecessary_wraps,
        reason = "tests use Result<()> uniformly so the ? operator can be added without churn"
    )]

    use anyhow::Result;
    use paddler_types::agent_desired_model::AgentDesiredModel;
    use paddler_types::huggingface_model_reference::HuggingFaceModelReference;

    use super::format_desired_model;

    #[test]
    fn formats_huggingface_reference_with_repo_filename_and_revision() -> Result<()> {
        let model = AgentDesiredModel::HuggingFace(HuggingFaceModelReference {
            repo_id: "org/repo".to_owned(),
            filename: "model.gguf".to_owned(),
            revision: "main".to_owned(),
        });

        assert_eq!(
            format_desired_model(&model),
            "HuggingFace org/repo/model.gguf (main)"
        );

        Ok(())
    }

    #[test]
    fn formats_local_to_agent_with_path_prefix() -> Result<()> {
        let model = AgentDesiredModel::LocalToAgent("/var/models/model.gguf".to_owned());

        assert_eq!(
            format_desired_model(&model),
            "Local: /var/models/model.gguf"
        );

        Ok(())
    }

    #[test]
    fn formats_none_as_not_set_placeholder() -> Result<()> {
        assert_eq!(format_desired_model(&AgentDesiredModel::None), "(not set)");

        Ok(())
    }
}
