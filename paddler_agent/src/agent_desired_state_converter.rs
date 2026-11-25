use std::ops::ControlFlow;
use std::path::PathBuf;
use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use paddler_agent_model_capabilities::model_capabilities_settings::ModelCapabilitiesSettings;
use paddler_agent_model_capabilities::multimodal_projection::MultimodalProjection;
use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_agent_text_generation::text_generation_settings::TextGenerationSettings;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::balancer_text_generation_settings::BalancerTextGenerationSettings;
use paddler_messaging::multimodal_settings::MultimodalSettings;
use paddler_model_source::desired_model_resolution::DesiredModelResolution;
use paddler_model_source::desired_model_resolver::DesiredModelResolver;

use crate::agent_applicable_state::AgentApplicableState;
use crate::agent_desired_state_conversion::AgentDesiredStateConversion;
use crate::agent_error::AgentError;
use crate::desired_model_file::DesiredModelFile;
use crate::model_serving_settings::ModelServingSettings;

pub struct AgentDesiredStateConverter {
    pub desired_model_resolver: DesiredModelResolver,
    pub cancellation_token: CancellationToken,
    pub slot_aggregated_status: Arc<SlotAggregatedStatus>,
}

impl AgentDesiredStateConverter {
    /// # Errors
    ///
    /// Returns [`AgentError::LocalFileMissing`] when the local file of the model or its multimodal projection does not exist, and [`AgentError::DesiredModelFileUnresolvable`] when one of their URIs cannot be resolved to a file.
    pub async fn convert(
        &self,
        AgentDesiredState {
            model,
            model_runtime_parameters,
            pooling_type,
            text_generation:
                BalancerTextGenerationSettings {
                    chat_template_source,
                    multimodal:
                        MultimodalSettings {
                            image_resize_to_fit,
                            projection,
                        },
                    sampling_parameters,
                },
        }: &AgentDesiredState,
    ) -> Result<AgentDesiredStateConversion, AgentError> {
        let ControlFlow::Continue(model_path) = self
            .resolve_model_file(model, DesiredModelFile::Model)
            .await?
        else {
            return Ok(AgentDesiredStateConversion::Cancelled);
        };
        let Some(model_path) = model_path else {
            return Ok(AgentDesiredStateConversion::Converted(
                AgentApplicableState::WithoutModel {
                    chat_template_source: chat_template_source.clone(),
                },
            ));
        };
        let ControlFlow::Continue(multimodal_projection_path) = self
            .resolve_model_file(projection, DesiredModelFile::MultimodalProjection)
            .await?
        else {
            return Ok(AgentDesiredStateConversion::Cancelled);
        };

        Ok(AgentDesiredStateConversion::Converted(
            AgentApplicableState::WithModel(ModelServingSettings {
                model_capabilities: ModelCapabilitiesSettings {
                    chat_template_source: chat_template_source.clone(),
                    embedding_pooling: pooling_type.clone(),
                    multimodal_projection: multimodal_projection_path.map_or(
                        MultimodalProjection::NotConfigured,
                        MultimodalProjection::File,
                    ),
                },
                model_path,
                model_runtime_parameters: model_runtime_parameters.clone(),
                text_generation: TextGenerationSettings {
                    image_resize_to_fit: *image_resize_to_fit,
                    sampling_parameters: *sampling_parameters,
                },
            }),
        ))
    }

    async fn resolve_model_file(
        &self,
        desired_model: &AgentDesiredModel,
        desired_model_file: DesiredModelFile,
    ) -> Result<ControlFlow<(), Option<PathBuf>>, AgentError> {
        match desired_model {
            AgentDesiredModel::None => Ok(ControlFlow::Continue(None)),
            AgentDesiredModel::Uri(uri) => Ok(
                match self.resolve_model_uri(uri, desired_model_file).await? {
                    ControlFlow::Break(()) => ControlFlow::Break(()),
                    ControlFlow::Continue(path) => ControlFlow::Continue(Some(path)),
                },
            ),
        }
    }

    async fn resolve_model_uri(
        &self,
        uri: &str,
        desired_model_file: DesiredModelFile,
    ) -> Result<ControlFlow<(), PathBuf>, AgentError> {
        match self
            .desired_model_resolver
            .resolve_uri(
                &self.cancellation_token,
                uri,
                self.slot_aggregated_status.clone(),
            )
            .await?
        {
            DesiredModelResolution::Cancelled => Ok(ControlFlow::Break(())),
            DesiredModelResolution::Resolved(path) => {
                desired_model_file.report_found(&path, &self.slot_aggregated_status);

                Ok(ControlFlow::Continue(path))
            }
            DesiredModelResolution::LocalFileMissing(path) => {
                desired_model_file.report_missing(&path, &self.slot_aggregated_status);

                Err(AgentError::LocalFileMissing { path })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs::write;
    use std::path::Path;
    use std::path::PathBuf;
    use std::sync::Arc;

    use tempfile::NamedTempFile;
    use tempfile::TempDir;
    use tempfile::tempdir;
    use tokio_util::sync::CancellationToken;

    use paddler_agent_model_capabilities::model_capabilities_settings::ModelCapabilitiesSettings;
    use paddler_agent_model_capabilities::multimodal_projection::MultimodalProjection;
    use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
    use paddler_agent_text_generation::text_generation_settings::TextGenerationSettings;
    use paddler_inference_parameters::desired_model_runtime_parameters::DesiredModelRuntimeParameters;
    use paddler_inference_parameters::desired_sampling_parameters::DesiredSamplingParameters;
    use paddler_inference_parameters::pooling_type::PoolingType;
    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::agent_desired_state::AgentDesiredState;
    use paddler_messaging::agent_issue::AgentIssue;
    use paddler_messaging::agent_issue_params::model_path::ModelPath;
    use paddler_messaging::balancer_text_generation_settings::BalancerTextGenerationSettings;
    use paddler_messaging::chat_template::ChatTemplate;
    use paddler_messaging::chat_template_source::ChatTemplateSource;
    use paddler_messaging::multimodal_settings::MultimodalSettings;
    use paddler_model_source::agent_local_model_path::AgentLocalModelPath;
    use paddler_model_source::constants::HUGGINGFACE_DEFAULT_ENDPOINT;
    use paddler_model_source::desired_model_resolver::DesiredModelResolver;
    use paddler_model_source::huggingface_model_reference::HuggingFaceModelReference;
    use paddler_model_source::model_source::ModelSource;

    use crate::agent_applicable_state::AgentApplicableState;
    use crate::agent_desired_state_conversion::AgentDesiredStateConversion;
    use crate::agent_desired_state_converter::AgentDesiredStateConverter;
    use crate::agent_error::AgentError;
    use crate::model_serving_settings::ModelServingSettings;

    fn process_env_desired_model_resolver() -> DesiredModelResolver {
        DesiredModelResolver::from_process_env(HUGGINGFACE_DEFAULT_ENDPOINT.to_owned()).unwrap()
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
            model,
            model_runtime_parameters: DesiredModelRuntimeParameters::default(),
            pooling_type: PoolingType::Last,
            text_generation: BalancerTextGenerationSettings {
                multimodal: MultimodalSettings {
                    projection: multimodal_projection,
                    ..MultimodalSettings::default()
                },
                ..BalancerTextGenerationSettings::default()
            },
        }
    }

    fn served_settings(
        model_file: &NamedTempFile,
        multimodal_projection: MultimodalProjection,
    ) -> AgentApplicableState {
        AgentApplicableState::WithModel(ModelServingSettings {
            model_capabilities: ModelCapabilitiesSettings {
                chat_template_source: ChatTemplateSource::default(),
                embedding_pooling: PoolingType::Last,
                multimodal_projection,
            },
            model_path: model_file.path().to_path_buf(),
            model_runtime_parameters: DesiredModelRuntimeParameters::default(),
            text_generation: TextGenerationSettings {
                image_resize_to_fit: MultimodalSettings::default().image_resize_to_fit,
                sampling_parameters: DesiredSamplingParameters::default(),
            },
        })
    }

    fn local_model_at(path: &Path) -> AgentDesiredModel {
        ModelSource::LocalToAgent(AgentLocalModelPath::from_absolute_path(path).unwrap())
            .into_agent_desired_model()
    }

    fn uncached_hugging_face_model() -> AgentDesiredModel {
        ModelSource::HuggingFace(HuggingFaceModelReference {
            filename: "uncached.gguf".to_owned(),
            repository_name: "repo".to_owned(),
            repository_owner: "owner".to_owned(),
            revision: "main".to_owned(),
        })
        .into_agent_desired_model()
    }

    async fn conversion(
        cancellation_token: CancellationToken,
        desired: &AgentDesiredState,
    ) -> AgentDesiredStateConversion {
        AgentDesiredStateConverter {
            desired_model_resolver: process_env_desired_model_resolver(),
            cancellation_token,
            slot_aggregated_status: Arc::new(SlotAggregatedStatus::new(1)),
        }
        .convert(desired)
        .await
        .expect("the desired state must convert")
    }

    async fn converted(desired: &AgentDesiredState) -> AgentDesiredStateConversion {
        conversion(CancellationToken::new(), desired).await
    }

    async fn cancelled_conversion(desired: &AgentDesiredState) -> AgentDesiredStateConversion {
        let cancellation_token = CancellationToken::new();

        cancellation_token.cancel();

        conversion(cancellation_token, desired).await
    }

    struct MissingLocalModel {
        _dir_guard: TempDir,
        path: PathBuf,
    }

    #[tokio::test]
    async fn local_missing_model_registers_model_file_does_not_exist_and_errs() {
        let status = Arc::new(SlotAggregatedStatus::new(1));
        let MissingLocalModel {
            _dir_guard,
            path: missing_path,
        } = nonexistent_path_in_temp_dir("model");
        let desired = desired_state(local_model_at(&missing_path), AgentDesiredModel::None);
        let converter = AgentDesiredStateConverter {
            desired_model_resolver: process_env_desired_model_resolver(),
            cancellation_token: CancellationToken::new(),
            slot_aggregated_status: status.clone(),
        };

        let outcome = converter.convert(&desired).await;

        assert!(
            matches!(&outcome, Err(AgentError::LocalFileMissing { path }) if *path == missing_path),
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
    async fn a_model_file_that_appears_on_a_retry_clears_its_missing_file_issue() {
        let status = Arc::new(SlotAggregatedStatus::new(1));
        let MissingLocalModel {
            _dir_guard,
            path: appearing_path,
        } = nonexistent_path_in_temp_dir("model");
        let desired = desired_state(local_model_at(&appearing_path), AgentDesiredModel::None);
        let converter = AgentDesiredStateConverter {
            desired_model_resolver: process_env_desired_model_resolver(),
            cancellation_token: CancellationToken::new(),
            slot_aggregated_status: status.clone(),
        };

        assert!(converter.convert(&desired).await.is_err());

        write(&appearing_path, b"").unwrap();
        converter.convert(&desired).await.unwrap();

        assert!(
            !status.has_issue(&AgentIssue::ModelFileDoesNotExist(ModelPath {
                model_path: appearing_path.display().to_string(),
            }))
        );
    }

    #[tokio::test]
    async fn local_missing_multimodal_projection_registers_multimodal_projection_cannot_be_loaded_and_errs()
     {
        let status = Arc::new(SlotAggregatedStatus::new(1));
        let model_file = NamedTempFile::new().unwrap();
        let MissingLocalModel {
            _dir_guard,
            path: missing_path,
        } = nonexistent_path_in_temp_dir("projection");
        let desired = desired_state(
            local_model_at(model_file.path()),
            local_model_at(&missing_path),
        );
        let converter = AgentDesiredStateConverter {
            desired_model_resolver: process_env_desired_model_resolver(),
            cancellation_token: CancellationToken::new(),
            slot_aggregated_status: status.clone(),
        };

        let outcome = converter.convert(&desired).await;

        assert!(
            matches!(&outcome, Err(AgentError::LocalFileMissing { path }) if *path == missing_path),
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
        let model_file = NamedTempFile::new().unwrap();

        assert_eq!(
            cancelled_conversion(&desired_state(
                local_model_at(model_file.path()),
                uncached_hugging_face_model()
            ))
            .await,
            AgentDesiredStateConversion::Cancelled
        );
    }

    #[tokio::test]
    async fn a_state_with_a_model_serves_text_generation_and_embeddings_with_its_resolved_files() {
        let model_file = NamedTempFile::new().unwrap();
        let projection_file = NamedTempFile::new().unwrap();

        assert_eq!(
            converted(&desired_state(
                local_model_at(model_file.path()),
                local_model_at(projection_file.path()),
            ))
            .await,
            AgentDesiredStateConversion::Converted(served_settings(
                &model_file,
                MultimodalProjection::File(projection_file.path().to_path_buf()),
            ))
        );
    }

    #[tokio::test]
    async fn a_state_without_a_model_keeps_its_chat_template() {
        let chat_template = ChatTemplate {
            content: "{{ messages }}".to_owned(),
        };

        assert_eq!(
            converted(&AgentDesiredState {
                text_generation: BalancerTextGenerationSettings {
                    chat_template_source: ChatTemplateSource::Override(chat_template.clone()),
                    ..BalancerTextGenerationSettings::default()
                },
                ..desired_state(AgentDesiredModel::None, AgentDesiredModel::None)
            })
            .await,
            AgentDesiredStateConversion::Converted(AgentApplicableState::WithoutModel {
                chat_template_source: ChatTemplateSource::Override(chat_template),
            })
        );
    }

    #[tokio::test]
    async fn a_state_without_a_model_does_not_resolve_its_multimodal_projection() {
        let MissingLocalModel {
            _dir_guard,
            path: missing_path,
        } = nonexistent_path_in_temp_dir("projection");

        assert_eq!(
            converted(&desired_state(
                AgentDesiredModel::None,
                local_model_at(&missing_path)
            ))
            .await,
            AgentDesiredStateConversion::Converted(AgentApplicableState::WithoutModel {
                chat_template_source: ChatTemplateSource::EmbeddedInModel,
            })
        );
    }
}
