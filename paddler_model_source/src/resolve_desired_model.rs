use std::sync::Arc;

use hf_hub::Cache;
use tokio_util::sync::CancellationToken;

use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_messaging::agent_desired_model::AgentDesiredModel;

use crate::desired_model_resolution::DesiredModelResolution;
use crate::huggingface_model_source::HuggingFaceModelSource;
use crate::local_model_path::LocalModelPath;
use crate::model_source_error::ModelSourceError;
use crate::resolves_model_source::ResolvesModelSource;
use crate::url_model_source::UrlModelSource;

pub async fn resolve_desired_model(
    cancellation_token: &CancellationToken,
    desired: &AgentDesiredModel,
    slot_aggregated_status: Arc<SlotAggregatedStatus>,
) -> Result<DesiredModelResolution, ModelSourceError> {
    match desired {
        AgentDesiredModel::HuggingFace(reference) => {
            HuggingFaceModelSource {
                cache: Cache::from_env(),
                reference: reference.clone(),
            }
            .resolve(cancellation_token, slot_aggregated_status)
            .await
        }
        AgentDesiredModel::LocalToAgent(path) => {
            LocalModelPath::new(path.clone())
                .resolve(cancellation_token, slot_aggregated_status)
                .await
        }
        AgentDesiredModel::Url(reference) => {
            UrlModelSource(reference.clone())
                .resolve(cancellation_token, slot_aggregated_status)
                .await
        }
        AgentDesiredModel::None => Ok(DesiredModelResolution::NotConfigured),
    }
}

#[cfg(test)]
mod tests {
    use std::mem;
    use std::sync::Arc;

    use tempfile::NamedTempFile;
    use tempfile::tempdir;
    use tokio_util::sync::CancellationToken;

    use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::agent_issue::AgentIssue;
    use paddler_messaging::huggingface_model_reference::HuggingFaceModelReference;
    use paddler_messaging::produces_snapshot::ProducesSnapshot;

    use crate::desired_model_resolution::DesiredModelResolution;
    use crate::model_source_error::ModelSourceError;
    use crate::resolve_desired_model::resolve_desired_model;

    fn fresh_status() -> Arc<SlotAggregatedStatus> {
        Arc::new(SlotAggregatedStatus::new(1))
    }

    #[tokio::test]
    async fn local_existing_file_resolves_to_resolved_with_that_path() {
        let status = fresh_status();
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();
        let desired = AgentDesiredModel::LocalToAgent(path.display().to_string());

        let resolution = resolve_desired_model(&CancellationToken::new(), &desired, status)
            .await
            .unwrap();

        assert!(matches!(
            resolution,
            DesiredModelResolution::Resolved(ref resolved) if *resolved == path
        ));
    }

    #[tokio::test]
    async fn local_missing_file_resolves_to_local_file_missing_with_that_path() {
        let status = fresh_status();
        let temp_dir = tempdir().unwrap();
        let path = temp_dir.path().join("missing-desired.gguf");
        let desired = AgentDesiredModel::LocalToAgent(path.display().to_string());

        let resolution = resolve_desired_model(&CancellationToken::new(), &desired, status)
            .await
            .unwrap();

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
        status.register_issue(AgentIssue::HuggingFaceModelDoesNotExist(
            reference.model_path(),
        ));
        let desired = AgentDesiredModel::HuggingFace(reference);

        let resolution = resolve_desired_model(&CancellationToken::new(), &desired, status).await;

        assert!(matches!(
            resolution,
            Err(ModelSourceError::HuggingFaceModelKnownToBeMissing { model_path })
                if model_path == "owner/repo/main/model.gguf"
        ));
    }

    #[tokio::test]
    async fn a_cancelled_token_aborts_a_huggingface_download_without_registering_an_issue() {
        let status = fresh_status();
        let reference = HuggingFaceModelReference {
            filename: "model.gguf".to_owned(),
            repo_id: "owner/repo".to_owned(),
            revision: "main".to_owned(),
        };
        let desired = AgentDesiredModel::HuggingFace(reference);
        let cancellation_token = CancellationToken::new();

        cancellation_token.cancel();

        let resolution = resolve_desired_model(&cancellation_token, &desired, status.clone()).await;

        assert!(
            resolution.is_ok_and(|resolution| resolution == DesiredModelResolution::Cancelled),
            "a cancelled Hugging Face download must report cancellation as an outcome, not an error"
        );
        assert!(
            status.make_snapshot().status.issues.is_empty(),
            "a cancelled Hugging Face download must not register a slot issue"
        );
    }

    #[tokio::test]
    async fn none_variant_resolves_to_not_configured() {
        let status = fresh_status();
        let desired = AgentDesiredModel::None;

        let resolution = resolve_desired_model(&CancellationToken::new(), &desired, status)
            .await
            .unwrap();

        assert_eq!(
            mem::discriminant(&resolution),
            mem::discriminant(&DesiredModelResolution::NotConfigured)
        );
    }
}
