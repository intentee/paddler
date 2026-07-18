use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use anyhow::anyhow;
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_state_conversion::converts_to_applicable_state::ConvertsToApplicableState;

use crate::agent_applicable_state::AgentApplicableState;
use crate::desired_model_resolution::DesiredModelResolution;
use crate::resolve_desired_model::resolve_desired_model;
use crate::slot_aggregated_status::SlotAggregatedStatus;

async fn resolve_into_optional_path<TLocalMissingIssue>(
    desired: &AgentDesiredModel,
    slot_aggregated_status: &Arc<SlotAggregatedStatus>,
    on_local_missing: TLocalMissingIssue,
    cancellation_token: &CancellationToken,
) -> Result<Option<PathBuf>>
where
    TLocalMissingIssue: FnOnce(ModelPath) -> AgentIssue,
{
    match resolve_desired_model(desired, slot_aggregated_status.clone(), cancellation_token).await?
    {
        DesiredModelResolution::NotConfigured => Ok(None),
        DesiredModelResolution::Resolved(path) => Ok(Some(path)),
        DesiredModelResolution::LocalFileMissing(path) => {
            let model_path_string = path.display().to_string();

            slot_aggregated_status.register_issue(on_local_missing(ModelPath {
                model_path: model_path_string.clone(),
            }));

            Err(anyhow!("Local file does not exist: {model_path_string}"))
        }
    }
}

pub struct AgentDesiredStateConverter {
    pub cancellation_token: CancellationToken,
    pub slot_aggregated_status: Arc<SlotAggregatedStatus>,
}

#[async_trait]
impl ConvertsToApplicableState for AgentDesiredStateConverter {
    type ApplicableState = AgentApplicableState;
    type DesiredState = AgentDesiredState;

    async fn to_applicable_state(
        &self,
        desired_state: AgentDesiredState,
    ) -> Result<AgentApplicableState> {
        let model_path = resolve_into_optional_path(
            &desired_state.model,
            &self.slot_aggregated_status,
            AgentIssue::ModelFileDoesNotExist,
            &self.cancellation_token,
        )
        .await?;

        let multimodal_projection_path = resolve_into_optional_path(
            &desired_state.multimodal_projection,
            &self.slot_aggregated_status,
            AgentIssue::MultimodalProjectionCannotBeLoaded,
            &self.cancellation_token,
        )
        .await?;

        Ok(AgentApplicableState {
            chat_template_override: desired_state.chat_template_override,
            inference_parameters: desired_state.inference_parameters,
            model_path,
            multimodal_projection_path,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use tempfile::TempDir;
    use tokio_util::sync::CancellationToken;

    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::agent_desired_state::AgentDesiredState;
    use paddler_messaging::agent_issue::AgentIssue;
    use paddler_messaging::agent_issue_params::model_path::ModelPath;
    use paddler_messaging::inference_parameters::InferenceParameters;
    use paddler_state_conversion::converts_to_applicable_state::ConvertsToApplicableState as _;

    use crate::agent_desired_state_converter::AgentDesiredStateConverter;
    use crate::slot_aggregated_status::SlotAggregatedStatus;

    struct MissingLocalModel {
        _dir_guard: TempDir,
        path: PathBuf,
    }

    fn fresh_status() -> Arc<SlotAggregatedStatus> {
        Arc::new(SlotAggregatedStatus::new(1))
    }

    fn nonexistent_path_in_temp_dir(label: &str) -> MissingLocalModel {
        let dir_guard = tempfile::tempdir().unwrap();
        let path = dir_guard.path().join(format!("missing-{label}.gguf"));

        MissingLocalModel {
            _dir_guard: dir_guard,
            path,
        }
    }

    fn desired_state(
        model: AgentDesiredModel,
        multimodal_projection: AgentDesiredModel,
    ) -> AgentDesiredState {
        AgentDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters::default(),
            model,
            multimodal_projection,
        }
    }

    #[tokio::test]
    async fn local_missing_model_registers_model_file_does_not_exist_and_errs() {
        let status = fresh_status();
        let MissingLocalModel {
            _dir_guard,
            path: missing_path,
        } = nonexistent_path_in_temp_dir("model");
        let desired = desired_state(
            AgentDesiredModel::LocalToAgent(missing_path.display().to_string()),
            AgentDesiredModel::None,
        );
        let converter = AgentDesiredStateConverter {
            cancellation_token: CancellationToken::new(),
            slot_aggregated_status: status.clone(),
        };

        let outcome = converter.to_applicable_state(desired).await;

        assert!(
            outcome.is_err(),
            "AgentDesiredStateConverter must Err when the model's local path is missing"
        );
        assert!(
            status.has_issue(&AgentIssue::ModelFileDoesNotExist(ModelPath {
                model_path: missing_path.display().to_string(),
            })),
            "ModelFileDoesNotExist must be registered for a missing local model file"
        );
        assert!(
            !status.has_issue(&AgentIssue::MultimodalProjectionCannotBeLoaded(ModelPath {
                model_path: missing_path.display().to_string(),
            })),
            "MultimodalProjectionCannotBeLoaded must NOT be registered for a missing model"
        );
    }

    #[tokio::test]
    async fn local_missing_multimodal_projection_registers_multimodal_projection_cannot_be_loaded_and_errs()
     {
        let status = fresh_status();
        let MissingLocalModel {
            _dir_guard,
            path: missing_path,
        } = nonexistent_path_in_temp_dir("projection");
        let desired = desired_state(
            AgentDesiredModel::None,
            AgentDesiredModel::LocalToAgent(missing_path.display().to_string()),
        );
        let converter = AgentDesiredStateConverter {
            cancellation_token: CancellationToken::new(),
            slot_aggregated_status: status.clone(),
        };

        let outcome = converter.to_applicable_state(desired).await;

        assert!(
            outcome.is_err(),
            "AgentDesiredStateConverter must Err when the projection's local path is missing"
        );
        assert!(
            status.has_issue(&AgentIssue::MultimodalProjectionCannotBeLoaded(ModelPath {
                model_path: missing_path.display().to_string(),
            })),
            "MultimodalProjectionCannotBeLoaded must be registered for a missing local projection file"
        );
        assert!(
            !status.has_issue(&AgentIssue::ModelFileDoesNotExist(ModelPath {
                model_path: missing_path.display().to_string(),
            })),
            "ModelFileDoesNotExist must NOT be registered for a missing projection"
        );
    }
}
