use std::ops::ControlFlow;
use std::path::PathBuf;
use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_agent_text_generation::multimodal_projection::MultimodalProjection;
use paddler_agent_text_generation::text_generation_settings::TextGenerationSettings;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::agent_inference_settings::AgentInferenceSettings;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::agent_text_generation_settings::AgentTextGenerationSettings;
use paddler_messaging::decision_settings::DecisionSettings;
use paddler_messaging::multimodal_settings::MultimodalSettings;
use paddler_model_source::desired_model_resolution::DesiredModelResolution;
use paddler_model_source::model_source_error::ModelSourceError;
use paddler_model_source::resolve_desired_model::resolve_desired_model;

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
            inference_settings,
            model,
            model_runtime_parameters,
        }: &AgentDesiredState,
    ) -> Result<AgentDesiredStateConversion, ModelSourceError> {
        let ControlFlow::Continue(model_path) = self
            .resolve_model_file(model, AgentIssue::ModelFileDoesNotExist)
            .await?
        else {
            return Ok(AgentDesiredStateConversion::Cancelled);
        };

        let applicable_state = match inference_settings {
            AgentInferenceSettings::Decision(DecisionSettings { pointer_head }) => {
                let ControlFlow::Continue(pointer_head_path) = self
                    .resolve_model_file(pointer_head, AgentIssue::PointerHeadCannotBeLoaded)
                    .await?
                else {
                    return Ok(AgentDesiredStateConversion::Cancelled);
                };

                match (model_path, pointer_head_path) {
                    (Some(model_path), Some(pointer_head_path)) => AgentApplicableState::Decision {
                        model_path,
                        model_runtime_parameters: model_runtime_parameters.clone(),
                        pointer_head_path,
                    },
                    _ => AgentApplicableState::NotConfigured,
                }
            }
            AgentInferenceSettings::Embeddings(embedding_parameters) => {
                model_path.map_or(AgentApplicableState::NotConfigured, |model_path| {
                    AgentApplicableState::Embeddings {
                        embedding_parameters: embedding_parameters.clone(),
                        model_path,
                        model_runtime_parameters: model_runtime_parameters.clone(),
                    }
                })
            }
            AgentInferenceSettings::TextGeneration(AgentTextGenerationSettings {
                chat_template_source,
                multimodal:
                    MultimodalSettings {
                        image_resize_to_fit,
                        projection,
                    },
                sampling_parameters,
            }) => {
                let ControlFlow::Continue(multimodal_projection_path) = self
                    .resolve_model_file(projection, AgentIssue::MultimodalProjectionCannotBeLoaded)
                    .await?
                else {
                    return Ok(AgentDesiredStateConversion::Cancelled);
                };

                model_path.map_or_else(
                    || AgentApplicableState::TextGenerationWithoutModel {
                        chat_template_source: chat_template_source.clone(),
                    },
                    |model_path| AgentApplicableState::TextGeneration {
                        model_path,
                        model_runtime_parameters: model_runtime_parameters.clone(),
                        text_generation_settings: TextGenerationSettings {
                            chat_template_source: chat_template_source.clone(),
                            image_resize_to_fit: *image_resize_to_fit,
                            multimodal_projection: multimodal_projection_path.map_or(
                                MultimodalProjection::NotConfigured,
                                MultimodalProjection::File,
                            ),
                            sampling_parameters: sampling_parameters.clone(),
                        },
                    },
                )
            }
        };

        Ok(AgentDesiredStateConversion::Converted(applicable_state))
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
    use std::path::PathBuf;
    use std::sync::Arc;

    use tempfile::NamedTempFile;
    use tempfile::TempDir;
    use tempfile::tempdir;
    use tokio_util::sync::CancellationToken;

    use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
    use paddler_agent_text_generation::multimodal_projection::MultimodalProjection;
    use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
    use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::agent_desired_state::AgentDesiredState;
    use paddler_messaging::agent_inference_settings::AgentInferenceSettings;
    use paddler_messaging::agent_issue::AgentIssue;
    use paddler_messaging::agent_issue_params::model_path::ModelPath;
    use paddler_messaging::agent_text_generation_settings::AgentTextGenerationSettings;
    use paddler_messaging::chat_template::ChatTemplate;
    use paddler_messaging::chat_template_source::ChatTemplateSource;
    use paddler_messaging::decision_settings::DecisionSettings;
    use paddler_messaging::multimodal_settings::MultimodalSettings;
    use paddler_model_source::huggingface_model_reference::HuggingFaceModelReference;
    use paddler_model_source::model_source::ModelSource;
    use paddler_model_source::model_source_error::ModelSourceError;

    use crate::agent_applicable_state::AgentApplicableState;
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
            inference_settings: AgentInferenceSettings::TextGeneration(
                AgentTextGenerationSettings {
                    multimodal: MultimodalSettings {
                        projection: multimodal_projection,
                        ..MultimodalSettings::default()
                    },
                    ..AgentTextGenerationSettings::default()
                },
            ),
            model,
            model_runtime_parameters: ModelRuntimeParameters::default(),
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
            ModelSource::LocalToAgent(missing_path.display().to_string())
                .into_agent_desired_model(),
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
            ModelSource::LocalToAgent(missing_path.display().to_string())
                .into_agent_desired_model(),
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
        assert_eq!(
            conversion(
                cancelled_token(),
                &desired_state(AgentDesiredModel::None, uncached_hugging_face_model()),
            )
            .await,
            Some(AgentDesiredStateConversion::Cancelled)
        );
    }

    #[tokio::test]
    async fn cancelling_while_the_pointer_head_resolves_cancels_the_conversion() {
        assert_eq!(
            conversion(
                cancelled_token(),
                &AgentDesiredState {
                    inference_settings: AgentInferenceSettings::Decision(DecisionSettings {
                        pointer_head: uncached_hugging_face_model(),
                    }),
                    model: AgentDesiredModel::None,
                    model_runtime_parameters: ModelRuntimeParameters::default(),
                },
            )
            .await,
            Some(AgentDesiredStateConversion::Cancelled)
        );
    }

    fn local_model(model_file: &NamedTempFile) -> AgentDesiredModel {
        ModelSource::LocalToAgent(model_file.path().display().to_string())
            .into_agent_desired_model()
    }

    fn cancelled_token() -> CancellationToken {
        let cancellation_token = CancellationToken::new();

        cancellation_token.cancel();

        cancellation_token
    }

    fn uncached_hugging_face_model() -> AgentDesiredModel {
        ModelSource::HuggingFace(HuggingFaceModelReference {
            filename: "uncached.gguf".to_owned(),
            repo_id: "owner/repo".to_owned(),
            revision: "main".to_owned(),
        })
        .into_agent_desired_model()
    }

    async fn conversion(
        cancellation_token: CancellationToken,
        desired: &AgentDesiredState,
    ) -> Option<AgentDesiredStateConversion> {
        AgentDesiredStateConverter {
            cancellation_token,
            slot_aggregated_status: fresh_status(),
        }
        .convert(desired)
        .await
        .ok()
    }

    async fn converted(desired: &AgentDesiredState) -> Option<AgentDesiredStateConversion> {
        conversion(CancellationToken::new(), desired).await
    }

    #[tokio::test]
    async fn an_embeddings_state_without_a_model_is_not_configured() {
        assert_eq!(
            converted(&AgentDesiredState {
                inference_settings: AgentInferenceSettings::Embeddings(
                    EmbeddingParameters::default()
                ),
                model: AgentDesiredModel::None,
                model_runtime_parameters: ModelRuntimeParameters::default(),
            })
            .await,
            Some(AgentDesiredStateConversion::Converted(
                AgentApplicableState::NotConfigured
            ))
        );
    }

    #[tokio::test]
    async fn an_embeddings_state_carries_its_resolved_model() {
        let model_file = NamedTempFile::new().unwrap();

        assert_eq!(
            converted(&AgentDesiredState {
                inference_settings: AgentInferenceSettings::Embeddings(
                    EmbeddingParameters::default()
                ),
                model: local_model(&model_file),
                model_runtime_parameters: ModelRuntimeParameters::default(),
            })
            .await,
            Some(AgentDesiredStateConversion::Converted(
                AgentApplicableState::Embeddings {
                    embedding_parameters: EmbeddingParameters::default(),
                    model_path: model_file.path().to_path_buf(),
                    model_runtime_parameters: ModelRuntimeParameters::default(),
                }
            ))
        );
    }

    #[tokio::test]
    async fn a_text_generation_state_carries_its_resolved_multimodal_projection() {
        let model_file = NamedTempFile::new().unwrap();
        let projection_file = NamedTempFile::new().unwrap();

        let conversion = converted(&desired_state(
            local_model(&model_file),
            local_model(&projection_file),
        ))
        .await;

        assert!(matches!(
            conversion,
            Some(AgentDesiredStateConversion::Converted(AgentApplicableState::TextGeneration {
                model_path,
                text_generation_settings,
                ..
            })) if model_path == model_file.path()
                && text_generation_settings.multimodal_projection
                    == MultimodalProjection::File(projection_file.path().to_path_buf())
        ));
    }

    #[tokio::test]
    async fn a_text_generation_state_without_a_model_keeps_its_chat_template() {
        let chat_template = ChatTemplate {
            content: "{{ messages }}".to_owned(),
        };

        assert_eq!(
            converted(&AgentDesiredState {
                inference_settings: AgentInferenceSettings::TextGeneration(
                    AgentTextGenerationSettings {
                        chat_template_source: ChatTemplateSource::Override(chat_template.clone()),
                        ..AgentTextGenerationSettings::default()
                    },
                ),
                model: AgentDesiredModel::None,
                model_runtime_parameters: ModelRuntimeParameters::default(),
            })
            .await,
            Some(AgentDesiredStateConversion::Converted(
                AgentApplicableState::TextGenerationWithoutModel {
                    chat_template_source: ChatTemplateSource::Override(chat_template),
                }
            ))
        );
    }

    #[tokio::test]
    async fn a_decision_state_carries_its_model_and_pointer_head() {
        let model_file = NamedTempFile::new().unwrap();
        let pointer_head_file = NamedTempFile::new().unwrap();

        assert_eq!(
            converted(&AgentDesiredState {
                inference_settings: AgentInferenceSettings::Decision(DecisionSettings {
                    pointer_head: local_model(&pointer_head_file),
                }),
                model: local_model(&model_file),
                model_runtime_parameters: ModelRuntimeParameters::default(),
            })
            .await,
            Some(AgentDesiredStateConversion::Converted(
                AgentApplicableState::Decision {
                    model_path: model_file.path().to_path_buf(),
                    model_runtime_parameters: ModelRuntimeParameters::default(),
                    pointer_head_path: pointer_head_file.path().to_path_buf(),
                }
            ))
        );
    }

    #[tokio::test]
    async fn a_decision_state_without_a_pointer_head_is_not_configured() {
        let model_file = NamedTempFile::new().unwrap();

        assert_eq!(
            converted(&AgentDesiredState {
                inference_settings: AgentInferenceSettings::Decision(DecisionSettings::default()),
                model: local_model(&model_file),
                model_runtime_parameters: ModelRuntimeParameters::default(),
            })
            .await,
            Some(AgentDesiredStateConversion::Converted(
                AgentApplicableState::NotConfigured
            ))
        );
    }
}
