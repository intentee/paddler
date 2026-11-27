use llama_cpp_bindings_types::model_capabilities::ModelCapabilities;
use llama_cpp_bindings_types::sampling_defaults::SamplingDefaults;
use llama_cpp_model_capabilities::detected_model_capabilities::DetectedModelCapabilities;
use llama_cpp_model_capabilities::model_capabilities_error::ModelCapabilitiesError;
use llama_cpp_model_capabilities::model_capabilities_sources::ModelCapabilitiesSources;

use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_agent_status::agent_issue_fix::AgentIssueFix;
use paddler_error_chain::error_chain::ErrorChain;
use paddler_inference_parameters::sampling_parameters::SamplingParameters;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::chat_template_does_not_compile_params::ChatTemplateDoesNotCompileParams;
use paddler_messaging::agent_issue_params::model_capability_failure_params::ModelCapabilityFailureParams;
use paddler_messaging::agent_issue_params::model_path::ModelPath;

use crate::agent_model_capabilities_error::AgentModelCapabilitiesError;
use crate::agent_pooling_type::AgentPoolingType;
use crate::conversation_chat_templates::ConversationChatTemplates;
use crate::model_capabilities_settings::ModelCapabilitiesSettings;
use crate::model_projector::ModelProjector;

const fn sampling_parameters_of(
    SamplingDefaults {
        min_p,
        penalty_frequency,
        penalty_last_n,
        penalty_presence,
        penalty_repeat,
        temperature,
        top_k,
        top_p,
    }: SamplingDefaults,
) -> SamplingParameters {
    SamplingParameters {
        min_p,
        penalty_frequency,
        penalty_last_n,
        penalty_presence,
        penalty_repeat,
        temperature,
        top_k,
        top_p,
    }
}

fn capability_failure_params(
    detection_error: &ModelCapabilitiesError,
    model_path: ModelPath,
) -> ModelCapabilityFailureParams {
    ModelCapabilityFailureParams {
        error: ErrorChain {
            error: detection_error,
        }
        .to_string(),
        model_path,
    }
}

fn detection_issue(detection_error: &ModelCapabilitiesError, model_path: ModelPath) -> AgentIssue {
    match detection_error {
        ModelCapabilitiesError::ChatTemplateCapabilitiesUnreadable(_) => {
            AgentIssue::ChatTemplateCapabilitiesUnreadable(capability_failure_params(
                detection_error,
                model_path,
            ))
        }
        ModelCapabilitiesError::ContextSizeUnrepresentable(_) => {
            AgentIssue::ModelTrainingContextIsUnrepresentable(model_path)
        }
        ModelCapabilitiesError::DeclaredPoolingTypeUnknown(_) => {
            AgentIssue::ModelDeclaresAnUnknownPoolingType(capability_failure_params(
                detection_error,
                model_path,
            ))
        }
        ModelCapabilitiesError::DecisionModelRejected(_) => {
            AgentIssue::DecisionModelCannotBeServed(capability_failure_params(
                detection_error,
                model_path,
            ))
        }
        ModelCapabilitiesError::PoolingProducesNoEmbeddingVectors { .. } => {
            AgentIssue::EmbeddingPoolingProducesNoVectors(model_path)
        }
    }
}

pub struct EstablishedModelCapabilities {
    pub chat_templates: ConversationChatTemplates,
    pub detected: DetectedModelCapabilities,
    pub projector: ModelProjector,
    pub reported: ModelCapabilities,
    pub sampling_defaults: SamplingParameters,
    pub settings: ModelCapabilitiesSettings,
}

impl EstablishedModelCapabilities {
    /// # Errors
    ///
    /// Returns the errors of [`ConversationChatTemplates::resolve`] when the chat template cannot be prepared, the errors of [`ModelProjector::load`] when the multimodal projection cannot be loaded, and [`AgentModelCapabilitiesError::CapabilitiesUndetectable`] when llama.cpp cannot tell what the model serves, registering the issue of each failure.
    pub fn establish(
        loaded_llama_model: &LoadedLlamaModel,
        inference_runtime_context: &InferenceRuntimeContext,
        settings: ModelCapabilitiesSettings,
    ) -> Result<Self, AgentModelCapabilitiesError> {
        let model_path = loaded_llama_model
            .llama_model_settings
            .reported_model_path();
        let slot_aggregated_status = &inference_runtime_context.slot_aggregated_status;
        let llama_model = &loaded_llama_model.model.llama_model;
        let chat_templates =
            match ConversationChatTemplates::resolve(&settings.chat_template_source, llama_model) {
                Ok(chat_templates) => {
                    slot_aggregated_status
                        .register_fix(&AgentIssueFix::ChatTemplateIsCompiled(model_path.clone()));

                    chat_templates
                }
                Err(template_failure) => {
                    slot_aggregated_status.register_pipeline_issue(
                        AgentIssue::ChatTemplateDoesNotCompile(ChatTemplateDoesNotCompileParams {
                            error: ErrorChain {
                                error: &template_failure,
                            }
                            .to_string(),
                            model_path,
                            template_source: settings.chat_template_source,
                        }),
                    );

                    return Err(template_failure);
                }
            };
        let projector = ModelProjector::load(
            &settings.multimodal_projection,
            loaded_llama_model,
            inference_runtime_context,
        )?;
        let detected = projector
            .with_multimodal_context(|multimodal_context| {
                DetectedModelCapabilities::detect(ModelCapabilitiesSources {
                    conversation_templates: chat_templates.conversation_templates(),
                    embedding_pooling: AgentPoolingType(settings.embedding_pooling.clone())
                        .embedding_pooling_choice(),
                    model: llama_model,
                    projector: multimodal_context,
                })
            })
            .map_err(|detection_error| {
                slot_aggregated_status
                    .register_pipeline_issue(detection_issue(&detection_error, model_path));

                AgentModelCapabilitiesError::CapabilitiesUndetectable(detection_error)
            })?;

        Ok(Self {
            chat_templates,
            reported: detected.report(),
            sampling_defaults: sampling_parameters_of(detected.model_defaults.sampling),
            detected,
            projector,
            settings,
        })
    }
}
