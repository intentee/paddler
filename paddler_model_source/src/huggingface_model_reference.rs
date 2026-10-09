use paddler_messaging::agent_issue_params::model_path::ModelPath;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HuggingFaceModelReference {
    pub filename: String,
    pub repo_id: String,
    pub revision: String,
}

impl HuggingFaceModelReference {
    #[must_use]
    pub fn model_path(&self) -> ModelPath {
        let Self {
            filename,
            repo_id,
            revision,
        } = self;

        ModelPath {
            model_path: format!("{repo_id}/{revision}/{filename}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::HuggingFaceModelReference;
    use paddler_messaging::agent_issue_params::model_path::ModelPath;

    #[test]
    fn names_the_model_path_after_its_repository_revision_and_filename() {
        let reference = HuggingFaceModelReference {
            filename: "model.gguf".to_owned(),
            repo_id: "owner/repo".to_owned(),
            revision: "v2".to_owned(),
        };

        assert_eq!(
            reference.model_path(),
            ModelPath {
                model_path: "owner/repo/v2/model.gguf".to_owned(),
            }
        );
    }
}
