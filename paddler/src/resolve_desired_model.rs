use std::sync::Arc;

use crate::agent_desired_model::AgentDesiredModel;
use anyhow::Result;

use crate::desired_model_resolution::DesiredModelResolution;
use crate::model_source::local::LocalModelPath;
use crate::resolves_model_source::ResolvesModelSource;
use crate::slot_aggregated_status::SlotAggregatedStatus;

pub async fn resolve_desired_model(
    desired: &AgentDesiredModel,
    slot_aggregated_status: Arc<SlotAggregatedStatus>,
) -> Result<DesiredModelResolution> {
    match desired {
        AgentDesiredModel::HuggingFace(reference) => {
            reference.resolve(slot_aggregated_status).await
        }
        AgentDesiredModel::LocalToAgent(path) => {
            LocalModelPath::new(path.clone())
                .resolve(slot_aggregated_status)
                .await
        }
        AgentDesiredModel::Url(reference) => reference.resolve(slot_aggregated_status).await,
        AgentDesiredModel::None => Ok(DesiredModelResolution::NotConfigured),
    }
}

#[cfg(test)]
mod tests {
    use std::mem;
    use std::sync::Arc;

    use crate::agent_desired_model::AgentDesiredModel;
    use tempfile::NamedTempFile;

    use crate::agent_issue::AgentIssue;
    use crate::agent_issue_params::ModelPath;
    use crate::desired_model_resolution::DesiredModelResolution;
    use crate::huggingface_model_reference::HuggingFaceModelReference;
    use crate::resolve_desired_model::resolve_desired_model;
    use crate::slot_aggregated_status::SlotAggregatedStatus;

    fn fresh_status() -> Arc<SlotAggregatedStatus> {
        Arc::new(SlotAggregatedStatus::new(1))
    }

    #[tokio::test]
    async fn local_existing_file_resolves_to_resolved_with_that_path() {
        let status = fresh_status();
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();
        let desired = AgentDesiredModel::LocalToAgent(path.display().to_string());

        let resolution = resolve_desired_model(&desired, status).await.unwrap();

        assert!(matches!(
            resolution,
            DesiredModelResolution::Resolved(ref resolved) if *resolved == path
        ));
    }

    #[tokio::test]
    async fn local_missing_file_resolves_to_local_file_missing_with_that_path() {
        let status = fresh_status();
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("missing-desired.gguf");
        let desired = AgentDesiredModel::LocalToAgent(path.display().to_string());

        let resolution = resolve_desired_model(&desired, status).await.unwrap();

        assert!(matches!(
            resolution,
            DesiredModelResolution::LocalFileMissing(ref missing) if *missing == path
        ));
    }

    #[tokio::test]
    async fn huggingface_already_marked_missing_resolves_to_error_without_network() {
        let status = fresh_status();
        let reference = HuggingFaceModelReference {
            filename: "model.gguf".to_owned(),
            repo_id: "owner/repo".to_owned(),
            revision: "main".to_owned(),
        };
        status.register_issue(AgentIssue::HuggingFaceModelDoesNotExist(ModelPath {
            model_path: "owner/repo/main/model.gguf".to_owned(),
        }));
        let desired = AgentDesiredModel::HuggingFace(reference);

        let resolution = resolve_desired_model(&desired, status).await;

        assert!(resolution.is_err());
    }

    #[tokio::test]
    async fn none_variant_resolves_to_not_configured() {
        let status = fresh_status();
        let desired = AgentDesiredModel::None;

        let resolution = resolve_desired_model(&desired, status).await.unwrap();

        assert_eq!(
            mem::discriminant(&resolution),
            mem::discriminant(&DesiredModelResolution::NotConfigured)
        );
    }
}
