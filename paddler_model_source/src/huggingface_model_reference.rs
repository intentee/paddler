use std::path::PathBuf;

use hf_hub::HFClient;
use hf_hub::HFRepository;
use hf_hub::HFResult;
use hf_hub::RepoTypeModel;

use paddler_messaging::agent_issue_params::model_path::ModelPath;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HuggingFaceModelReference {
    pub filename: String,
    pub repository_name: String,
    pub repository_owner: String,
    pub revision: String,
}

impl HuggingFaceModelReference {
    #[must_use]
    pub fn model_path(&self) -> ModelPath {
        let Self {
            filename,
            repository_name,
            repository_owner,
            revision,
        } = self;

        ModelPath {
            model_path: format!("{repository_owner}/{repository_name}/{revision}/{filename}"),
        }
    }

    #[must_use]
    pub fn repository(&self, client: &HFClient) -> HFRepository<RepoTypeModel> {
        client.model(self.repository_owner.clone(), self.repository_name.clone())
    }

    pub async fn cached_path(&self, repository: &HFRepository<RepoTypeModel>) -> HFResult<PathBuf> {
        repository
            .download_file()
            .filename(self.filename.clone())
            .revision(self.revision.clone())
            .local_files_only(true)
            .send()
            .await
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
            repository_name: "repo".to_owned(),
            repository_owner: "owner".to_owned(),
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
