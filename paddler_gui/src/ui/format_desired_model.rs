use paddler_types::agent_desired_model::AgentDesiredModel;

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
    use anyhow::Result;
    use anyhow::bail;
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

        if format_desired_model(&model) != "HuggingFace org/repo/model.gguf (main)" {
            bail!("HuggingFace formatting does not match the expected layout");
        }

        Ok(())
    }

    #[test]
    fn formats_local_to_agent_with_path_prefix() -> Result<()> {
        let model = AgentDesiredModel::LocalToAgent("/var/models/model.gguf".to_owned());

        if format_desired_model(&model) != "Local: /var/models/model.gguf" {
            bail!("LocalToAgent formatting does not match the expected layout");
        }

        Ok(())
    }

    #[test]
    fn formats_none_as_not_set_placeholder() -> Result<()> {
        if format_desired_model(&AgentDesiredModel::None) != "(not set)" {
            bail!("None formatting does not match the expected placeholder");
        }

        Ok(())
    }
}
