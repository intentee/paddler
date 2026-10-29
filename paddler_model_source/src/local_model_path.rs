use std::path::PathBuf;
use std::sync::Arc;

use tokio::fs::try_exists;
use tokio_util::sync::CancellationToken;

use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;

use crate::desired_model_resolution::DesiredModelResolution;
use crate::model_source_error::ModelSourceError;
use crate::resolves_model_source::ResolvesModelSource;

pub struct LocalModelPath {
    pub path: PathBuf,
}

impl ResolvesModelSource for LocalModelPath {
    async fn resolve(
        &self,
        _cancellation_token: &CancellationToken,
        slot_aggregated_status: Arc<SlotAggregatedStatus>,
    ) -> Result<DesiredModelResolution, ModelSourceError> {
        match try_exists(&self.path).await {
            Ok(true) => Ok(DesiredModelResolution::Resolved(self.path.clone())),
            Ok(false) => Ok(DesiredModelResolution::LocalFileMissing(self.path.clone())),
            Err(source) => {
                slot_aggregated_status.register_desired_state_issue(
                    AgentIssue::LocalModelFileCannotBeChecked(ModelPath::from(self.path.as_path())),
                );

                Err(ModelSourceError::LocalFileUncheckable {
                    path: self.path.clone(),
                    source,
                })
            }
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::fs::metadata;
    use std::sync::Arc;

    use tempfile::NamedTempFile;
    use tokio_util::sync::CancellationToken;

    use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
    use paddler_messaging::agent_issue::AgentIssue;
    use paddler_messaging::agent_issue_params::model_path::ModelPath;

    use super::LocalModelPath;
    use crate::model_source_error::ModelSourceError;
    use crate::resolves_model_source::ResolvesModelSource as _;

    #[tokio::test]
    async fn a_model_path_below_a_regular_file_cannot_be_checked() {
        let regular_file = NamedTempFile::new().unwrap();
        let path_below_the_regular_file = regular_file.path().join("model.gguf");

        assert_eq!(
            LocalModelPath {
                path: path_below_the_regular_file.clone(),
            }
            .resolve(
                &CancellationToken::new(),
                Arc::new(SlotAggregatedStatus::new(1))
            )
            .await
            .err()
            .map(|resolution_error| resolution_error.to_string()),
            Some(
                ModelSourceError::LocalFileUncheckable {
                    path: path_below_the_regular_file.clone(),
                    source: metadata(&path_below_the_regular_file).unwrap_err(),
                }
                .to_string()
            )
        );
    }

    #[tokio::test]
    async fn a_model_path_that_cannot_be_checked_is_reported_as_an_agent_issue() {
        let regular_file = NamedTempFile::new().unwrap();
        let path_below_the_regular_file = regular_file.path().join("model.gguf");
        let slot_aggregated_status = Arc::new(SlotAggregatedStatus::new(1));

        LocalModelPath {
            path: path_below_the_regular_file.clone(),
        }
        .resolve(&CancellationToken::new(), slot_aggregated_status.clone())
        .await
        .unwrap_err();

        assert!(
            slot_aggregated_status.has_issue(&AgentIssue::LocalModelFileCannotBeChecked(
                ModelPath::from(path_below_the_regular_file.as_path())
            ))
        );
    }
}
