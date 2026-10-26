use std::ops::ControlFlow;
use std::path::PathBuf;
use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_agent_text_generation::multimodal_projection::MultimodalProjection;
use paddler_agent_text_generation::text_generation_settings::TextGenerationSettings;
use paddler_messaging::agent_decisions_settings::AgentDecisionsSettings;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::balancer_text_generation_settings::BalancerTextGenerationSettings;
use paddler_messaging::decision_settings::DecisionSettings;
use paddler_messaging::multimodal_settings::MultimodalSettings;
use paddler_model_source::desired_model_resolution::DesiredModelResolution;
use paddler_model_source::desired_model_resolver::DesiredModelResolver;
use paddler_model_source::model_source_error::ModelSourceError;

use crate::agent_applicable_state::AgentApplicableState;
use crate::agent_desired_state_conversion::AgentDesiredStateConversion;
use crate::applicable_decisions::ApplicableDecisions;
use crate::desired_model_file::DesiredModelFile;
use crate::inference_capabilities::InferenceCapabilities;
use crate::model_serving_settings::ModelServingSettings;

pub struct AgentDesiredStateConverter {
    pub desired_model_resolver: DesiredModelResolver,
    pub cancellation_token: CancellationToken,
    pub slot_aggregated_status: Arc<SlotAggregatedStatus>,
}

impl AgentDesiredStateConverter {
    pub async fn convert(
        &self,
        AgentDesiredState {
            decisions,
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
    ) -> Result<AgentDesiredStateConversion, ModelSourceError> {
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
        let applicable_decisions = match self.resolve_decisions(decisions).await? {
            ControlFlow::Continue(applicable_decisions) => applicable_decisions,
            ControlFlow::Break(conversion_without_decisions) => {
                return Ok(conversion_without_decisions);
            }
        };

        Ok(AgentDesiredStateConversion::Converted(
            AgentApplicableState::WithModel(ModelServingSettings {
                capabilities: InferenceCapabilities {
                    decisions: applicable_decisions,
                    embeddings: pooling_type.clone(),
                    text_generation: TextGenerationSettings {
                        chat_template_source: chat_template_source.clone(),
                        image_resize_to_fit: *image_resize_to_fit,
                        multimodal_projection: multimodal_projection_path.map_or(
                            MultimodalProjection::NotConfigured,
                            MultimodalProjection::File,
                        ),
                        sampling_parameters: sampling_parameters.clone(),
                    },
                },
                model_path,
                model_runtime_parameters: model_runtime_parameters.clone(),
            }),
        ))
    }

    async fn resolve_decisions(
        &self,
        decisions: &AgentDecisionsSettings,
    ) -> Result<ControlFlow<AgentDesiredStateConversion, ApplicableDecisions>, ModelSourceError>
    {
        match decisions {
            AgentDecisionsSettings::NotServed => {
                Ok(ControlFlow::Continue(ApplicableDecisions::Disabled))
            }
            AgentDecisionsSettings::Served(DecisionSettings { pointer_head }) => Ok(
                match self
                    .resolve_model_file(pointer_head, DesiredModelFile::PointerHead)
                    .await?
                {
                    ControlFlow::Break(()) => {
                        ControlFlow::Break(AgentDesiredStateConversion::Cancelled)
                    }
                    ControlFlow::Continue(None) => ControlFlow::Break(
                        AgentDesiredStateConversion::Converted(AgentApplicableState::NotConfigured),
                    ),
                    ControlFlow::Continue(Some(pointer_head_path)) => {
                        ControlFlow::Continue(ApplicableDecisions::Enabled { pointer_head_path })
                    }
                },
            ),
        }
    }

    async fn resolve_model_file(
        &self,
        desired_model: &AgentDesiredModel,
        desired_model_file: DesiredModelFile,
    ) -> Result<ControlFlow<(), Option<PathBuf>>, ModelSourceError> {
        match self
            .desired_model_resolver
            .resolve(
                &self.cancellation_token,
                desired_model,
                self.slot_aggregated_status.clone(),
            )
            .await?
        {
            DesiredModelResolution::Cancelled => Ok(ControlFlow::Break(())),
            DesiredModelResolution::NotConfigured => Ok(ControlFlow::Continue(None)),
            DesiredModelResolution::Resolved(path) => {
                desired_model_file.report_found(&path, &self.slot_aggregated_status);

                Ok(ControlFlow::Continue(Some(path)))
            }
            DesiredModelResolution::LocalFileMissing(path) => {
                desired_model_file.report_missing(&path, &self.slot_aggregated_status);

                Err(ModelSourceError::LocalFileMissing { path })
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

    use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
    use paddler_agent_text_generation::multimodal_projection::MultimodalProjection;
    use paddler_agent_text_generation::text_generation_settings::TextGenerationSettings;
    use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
    use paddler_inference_parameters::pooling_type::PoolingType;
    use paddler_inference_parameters::sampling_parameters::SamplingParameters;
    use paddler_messaging::agent_decisions_settings::AgentDecisionsSettings;
    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::agent_desired_state::AgentDesiredState;
    use paddler_messaging::agent_issue::AgentIssue;
    use paddler_messaging::agent_issue_params::model_path::ModelPath;
    use paddler_messaging::balancer_text_generation_settings::BalancerTextGenerationSettings;
    use paddler_messaging::chat_template::ChatTemplate;
    use paddler_messaging::chat_template_source::ChatTemplateSource;
    use paddler_messaging::decision_settings::DecisionSettings;
    use paddler_messaging::multimodal_settings::MultimodalSettings;
    use paddler_model_source::agent_local_model_path::AgentLocalModelPath;
    use paddler_model_source::desired_model_resolver::DesiredModelResolver;
    use paddler_model_source::huggingface_default_endpoint::HUGGINGFACE_DEFAULT_ENDPOINT;
    use paddler_model_source::huggingface_model_reference::HuggingFaceModelReference;
    use paddler_model_source::model_source::ModelSource;
    use paddler_model_source::model_source_error::ModelSourceError;

    use crate::agent_applicable_state::AgentApplicableState;
    use crate::agent_desired_state_conversion::AgentDesiredStateConversion;
    use crate::agent_desired_state_converter::AgentDesiredStateConverter;
    use crate::applicable_decisions::ApplicableDecisions;
    use crate::inference_capabilities::InferenceCapabilities;
    use crate::model_serving_settings::ModelServingSettings;

    struct MissingLocalModel {
        _dir_guard: TempDir,
        path: PathBuf,
    }

    fn process_env_desired_model_resolver() -> DesiredModelResolver {
        DesiredModelResolver::from_process_env(HUGGINGFACE_DEFAULT_ENDPOINT.to_owned()).unwrap()
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
            decisions: AgentDecisionsSettings::NotServed,
            model,
            model_runtime_parameters: ModelRuntimeParameters::default(),
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

    fn desired_state_serving_decisions(
        model: AgentDesiredModel,
        pointer_head: AgentDesiredModel,
    ) -> AgentDesiredState {
        AgentDesiredState {
            decisions: AgentDecisionsSettings::Served(DecisionSettings { pointer_head }),
            ..desired_state(model, AgentDesiredModel::None)
        }
    }

    fn served_settings(
        model_file: &NamedTempFile,
        decisions: ApplicableDecisions,
        multimodal_projection: MultimodalProjection,
    ) -> AgentApplicableState {
        AgentApplicableState::WithModel(ModelServingSettings {
            capabilities: InferenceCapabilities {
                decisions,
                embeddings: PoolingType::Last,
                text_generation: TextGenerationSettings {
                    chat_template_source: ChatTemplateSource::default(),
                    image_resize_to_fit: MultimodalSettings::default().image_resize_to_fit,
                    multimodal_projection,
                    sampling_parameters: SamplingParameters::default(),
                },
            },
            model_path: model_file.path().to_path_buf(),
            model_runtime_parameters: ModelRuntimeParameters::default(),
        })
    }

    fn local_model_at(path: &Path) -> AgentDesiredModel {
        ModelSource::LocalToAgent(AgentLocalModelPath::from_absolute_path(path).unwrap())
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
            slot_aggregated_status: fresh_status(),
        }
        .convert(desired)
        .await
        .expect("the desired state must convert")
    }

    async fn converted(desired: &AgentDesiredState) -> AgentDesiredStateConversion {
        conversion(CancellationToken::new(), desired).await
    }

    #[tokio::test]
    async fn local_missing_model_registers_model_file_does_not_exist_and_errs() {
        let status = fresh_status();
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
    async fn a_model_file_that_appears_on_a_retry_clears_its_missing_file_issue() {
        let status = fresh_status();
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
    async fn a_pointer_head_that_appears_on_a_retry_clears_its_missing_file_issue() {
        let status = fresh_status();
        let model_file = NamedTempFile::new().unwrap();
        let MissingLocalModel {
            _dir_guard,
            path: appearing_path,
        } = nonexistent_path_in_temp_dir("pointer-head");
        let desired = desired_state_serving_decisions(
            local_model_at(model_file.path()),
            local_model_at(&appearing_path),
        );
        let converter = AgentDesiredStateConverter {
            desired_model_resolver: process_env_desired_model_resolver(),
            cancellation_token: CancellationToken::new(),
            slot_aggregated_status: status.clone(),
        };

        assert!(converter.convert(&desired).await.is_err());

        write(&appearing_path, b"").unwrap();
        converter.convert(&desired).await.unwrap();

        assert!(
            !status.has_issue(&AgentIssue::PointerHeadCannotBeLoaded(ModelPath {
                model_path: appearing_path.display().to_string(),
            }))
        );
    }

    #[tokio::test]
    async fn local_missing_multimodal_projection_registers_multimodal_projection_cannot_be_loaded_and_errs()
     {
        let status = fresh_status();
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
    async fn local_missing_pointer_head_registers_pointer_head_cannot_be_loaded_and_errs() {
        let status = fresh_status();
        let model_file = NamedTempFile::new().unwrap();
        let MissingLocalModel {
            _dir_guard,
            path: missing_path,
        } = nonexistent_path_in_temp_dir("pointer-head");
        let converter = AgentDesiredStateConverter {
            desired_model_resolver: process_env_desired_model_resolver(),
            cancellation_token: CancellationToken::new(),
            slot_aggregated_status: status.clone(),
        };

        let outcome = converter
            .convert(&desired_state_serving_decisions(
                local_model_at(model_file.path()),
                local_model_at(&missing_path),
            ))
            .await;

        assert_eq!(
            outcome.err().as_ref().map(ToString::to_string),
            Some(
                ModelSourceError::LocalFileMissing {
                    path: missing_path.clone(),
                }
                .to_string()
            )
        );
        assert!(
            status.has_issue(&AgentIssue::PointerHeadCannotBeLoaded(ModelPath {
                model_path: missing_path.display().to_string(),
            })),
            "PointerHeadCannotBeLoaded must be registered for a missing local pointer head"
        );
    }

    #[tokio::test]
    async fn cancelling_while_the_multimodal_projection_resolves_cancels_the_conversion() {
        let model_file = NamedTempFile::new().unwrap();

        assert_eq!(
            conversion(
                cancelled_token(),
                &desired_state(
                    local_model_at(model_file.path()),
                    uncached_hugging_face_model()
                ),
            )
            .await,
            AgentDesiredStateConversion::Cancelled
        );
    }

    #[tokio::test]
    async fn cancelling_while_the_pointer_head_resolves_cancels_the_conversion() {
        let model_file = NamedTempFile::new().unwrap();

        assert_eq!(
            conversion(
                cancelled_token(),
                &desired_state_serving_decisions(
                    local_model_at(model_file.path()),
                    uncached_hugging_face_model(),
                ),
            )
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
                ApplicableDecisions::Disabled,
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

    #[tokio::test]
    async fn a_state_serving_decisions_carries_its_pointer_head() {
        let model_file = NamedTempFile::new().unwrap();
        let pointer_head_file = NamedTempFile::new().unwrap();

        assert_eq!(
            converted(&desired_state_serving_decisions(
                local_model_at(model_file.path()),
                local_model_at(pointer_head_file.path()),
            ))
            .await,
            AgentDesiredStateConversion::Converted(served_settings(
                &model_file,
                ApplicableDecisions::Enabled {
                    pointer_head_path: pointer_head_file.path().to_path_buf(),
                },
                MultimodalProjection::NotConfigured,
            ))
        );
    }

    #[tokio::test]
    async fn a_state_serving_decisions_without_a_pointer_head_is_not_configured() {
        let model_file = NamedTempFile::new().unwrap();

        assert_eq!(
            converted(&desired_state_serving_decisions(
                local_model_at(model_file.path()),
                AgentDesiredModel::None,
            ))
            .await,
            AgentDesiredStateConversion::Converted(AgentApplicableState::NotConfigured)
        );
    }
}
