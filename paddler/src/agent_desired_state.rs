use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use anyhow::anyhow;
use async_trait::async_trait;
use serde::Deserialize;
use serde::Serialize;

use crate::agent_applicable_state::AgentApplicableState;
use crate::agent_desired_model::AgentDesiredModel;
use crate::agent_issue::AgentIssue;
use crate::agent_issue_params::ModelPath;
use crate::chat_template::ChatTemplate;
use crate::converts_to_applicable_state::ConvertsToApplicableState;
use crate::desired_model_resolution::DesiredModelResolution;
use crate::inference_parameters::InferenceParameters;
use crate::resolve_desired_model::resolve_desired_model;
use crate::slot_aggregated_status::SlotAggregatedStatus;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgentDesiredState {
    pub chat_template_override: Option<ChatTemplate>,
    pub inference_parameters: InferenceParameters,
    pub model: AgentDesiredModel,
    pub multimodal_projection: AgentDesiredModel,
}

async fn resolve_into_optional_path<TLocalMissingIssue>(
    desired: &AgentDesiredModel,
    slot_aggregated_status: &Arc<SlotAggregatedStatus>,
    on_local_missing: TLocalMissingIssue,
) -> Result<Option<PathBuf>>
where
    TLocalMissingIssue: FnOnce(ModelPath) -> AgentIssue,
{
    match resolve_desired_model(desired, slot_aggregated_status.clone()).await? {
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

#[async_trait]
impl ConvertsToApplicableState for AgentDesiredState {
    type ApplicableState = AgentApplicableState;
    type Context = Arc<SlotAggregatedStatus>;

    async fn to_applicable_state(
        &self,
        slot_aggregated_status: Self::Context,
    ) -> Result<Self::ApplicableState> {
        let model_path = resolve_into_optional_path(
            &self.model,
            &slot_aggregated_status,
            AgentIssue::ModelFileDoesNotExist,
        )
        .await?;

        let multimodal_projection_path = resolve_into_optional_path(
            &self.multimodal_projection,
            &slot_aggregated_status,
            AgentIssue::MultimodalProjectionCannotBeLoaded,
        )
        .await?;

        Ok(AgentApplicableState {
            chat_template_override: self.chat_template_override.clone(),
            inference_parameters: self.inference_parameters.clone(),
            model_path,
            multimodal_projection_path,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use anyhow::Result;
    use tempfile::TempDir;

    use crate::agent_desired_model::AgentDesiredModel;
    use crate::agent_desired_state::AgentDesiredState;
    use crate::agent_issue::AgentIssue;
    use crate::agent_issue_params::ModelPath;
    use crate::converts_to_applicable_state::ConvertsToApplicableState;
    use crate::inference_parameters::InferenceParameters;
    use crate::slot_aggregated_status::SlotAggregatedStatus;

    fn fresh_status() -> Arc<SlotAggregatedStatus> {
        Arc::new(SlotAggregatedStatus::new(1))
    }

    fn nonexistent_path_in_temp_dir(label: &str) -> Result<(TempDir, PathBuf)> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join(format!("missing-{label}.gguf"));

        Ok((dir, path))
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
    async fn local_missing_model_registers_model_file_does_not_exist_and_errs() -> Result<()> {
        let status = fresh_status();
        let (_dir_guard, missing_path) = nonexistent_path_in_temp_dir("model")?;
        let desired = desired_state(
            AgentDesiredModel::LocalToAgent(missing_path.display().to_string()),
            AgentDesiredModel::None,
        );

        let outcome = desired.to_applicable_state(status.clone()).await;

        assert!(
            outcome.is_err(),
            "AgentDesiredState::to_applicable_state must Err when the model's local path is missing"
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

        Ok(())
    }

    #[tokio::test]
    async fn local_missing_multimodal_projection_registers_multimodal_projection_cannot_be_loaded_and_errs()
    -> Result<()> {
        let status = fresh_status();
        let (_dir_guard, missing_path) = nonexistent_path_in_temp_dir("projection")?;
        let desired = desired_state(
            AgentDesiredModel::None,
            AgentDesiredModel::LocalToAgent(missing_path.display().to_string()),
        );

        let outcome = desired.to_applicable_state(status.clone()).await;

        assert!(
            outcome.is_err(),
            "AgentDesiredState::to_applicable_state must Err when the projection's local path is missing"
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

        Ok(())
    }
}
