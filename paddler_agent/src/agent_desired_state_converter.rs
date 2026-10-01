use std::ops::ControlFlow;
use std::path::PathBuf;
use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_model_source::desired_model_resolution::DesiredModelResolution;
use paddler_model_source::model_source_error::ModelSourceError;
use paddler_model_source::resolve_desired_model::resolve_desired_model;

use crate::agent_applicable_model::AgentApplicableModel;
use crate::agent_applicable_state::AgentApplicableState;
use crate::agent_desired_state_conversion::AgentDesiredStateConversion;

pub struct AgentDesiredStateConverter {
    pub cancellation_token: CancellationToken,
    pub slot_aggregated_status: Arc<SlotAggregatedStatus>,
}

impl AgentDesiredStateConverter {
    pub async fn convert(
        &self,
        AgentDesiredState {
            chat_template_override,
            inference_parameters,
            model,
            multimodal_projection,
        }: &AgentDesiredState,
    ) -> Result<AgentDesiredStateConversion, ModelSourceError> {
        let ControlFlow::Continue(model_path) = self
            .resolve_model_file(model, AgentIssue::ModelFileDoesNotExist)
            .await?
        else {
            return Ok(AgentDesiredStateConversion::Cancelled);
        };
        let ControlFlow::Continue(multimodal_projection_path) = self
            .resolve_model_file(
                multimodal_projection,
                AgentIssue::MultimodalProjectionCannotBeLoaded,
            )
            .await?
        else {
            return Ok(AgentDesiredStateConversion::Cancelled);
        };

        Ok(AgentDesiredStateConversion::Converted(
            AgentApplicableState {
                chat_template_override: chat_template_override.clone(),
                inference_parameters: inference_parameters.clone(),
                model: model_path.map_or(AgentApplicableModel::NotConfigured, |model_path| {
                    AgentApplicableModel::Resolved {
                        model_path,
                        multimodal_projection_path,
                    }
                }),
            },
        ))
    }

    async fn resolve_model_file(
        &self,
        desired_model: &AgentDesiredModel,
        local_file_missing_issue: fn(ModelPath) -> AgentIssue,
    ) -> Result<ControlFlow<(), Option<PathBuf>>, ModelSourceError> {
        match resolve_desired_model(
            &self.cancellation_token,
            desired_model,
            self.slot_aggregated_status.clone(),
        )
        .await?
        {
            DesiredModelResolution::Cancelled => Ok(ControlFlow::Break(())),
            DesiredModelResolution::NotConfigured => Ok(ControlFlow::Continue(None)),
            DesiredModelResolution::Resolved(path) => Ok(ControlFlow::Continue(Some(path))),
            DesiredModelResolution::LocalFileMissing(path) => {
                self.slot_aggregated_status
                    .register_issue(local_file_missing_issue(ModelPath::from(path.as_path())));

                Err(ModelSourceError::LocalFileMissing { path })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;
    use std::path::PathBuf;
    use std::sync::Arc;

    use tempfile::TempDir;
    use tempfile::tempdir;
    use tokio_util::sync::CancellationToken;

    use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
    use paddler_inference_parameters::inference_parameters::InferenceParameters;
    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::agent_desired_state::AgentDesiredState;
    use paddler_messaging::agent_issue::AgentIssue;
    use paddler_messaging::agent_issue_params::model_path::ModelPath;
    use paddler_messaging::huggingface_model_reference::HuggingFaceModelReference;
    use paddler_model_source::model_source_error::ModelSourceError;

    use crate::agent_desired_state_conversion::AgentDesiredStateConversion;
    use crate::agent_desired_state_converter::AgentDesiredStateConverter;

    struct MissingLocalModel {
        _dir_guard: TempDir,
        path: PathBuf,
    }

    fn fresh_status() -> Arc<SlotAggregatedStatus> {
        Arc::new(SlotAggregatedStatus::new(1))
    }

    fn nonexistent_path_in_temp_dir(label: &str) -> MissingLocalModel {
        let dir_guard = tempdir().unwrap();
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

        let outcome = converter.convert(&desired).await;

        assert!(
            matches!(&outcome, Err(ModelSourceError::LocalFileMissing { path }) if *path == missing_path),
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

        let outcome = converter.convert(&desired).await;

        assert!(
            matches!(&outcome, Err(ModelSourceError::LocalFileMissing { path }) if *path == missing_path),
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

    #[tokio::test]
    async fn cancelling_while_the_multimodal_projection_resolves_cancels_the_conversion() {
        let cancellation_token = CancellationToken::new();
        let desired = desired_state(
            AgentDesiredModel::None,
            AgentDesiredModel::HuggingFace(HuggingFaceModelReference {
                filename: "projection.gguf".to_owned(),
                repo_id: "owner/repo".to_owned(),
                revision: "main".to_owned(),
            }),
        );

        cancellation_token.cancel();

        let outcome = AgentDesiredStateConverter {
            cancellation_token,
            slot_aggregated_status: fresh_status(),
        }
        .convert(&desired)
        .await;

        assert!(outcome.is_ok_and(|conversion| {
            discriminant(&conversion) == discriminant(&AgentDesiredStateConversion::Cancelled)
        }));
    }
}
