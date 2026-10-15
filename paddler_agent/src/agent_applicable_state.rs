use std::path::PathBuf;

use paddler_agent_text_generation::text_generation_settings::TextGenerationSettings;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::chat_template_source::ChatTemplateSource;

#[derive(Clone, Debug, Default, PartialEq)]
pub enum AgentApplicableState {
    Decision {
        model_path: PathBuf,
        model_runtime_parameters: ModelRuntimeParameters,
        pointer_head_path: PathBuf,
    },
    Embeddings {
        model_path: PathBuf,
        model_runtime_parameters: ModelRuntimeParameters,
        pooling_type: PoolingType,
    },
    #[default]
    NotConfigured,
    TextGeneration {
        model_path: PathBuf,
        model_runtime_parameters: ModelRuntimeParameters,
        text_generation_settings: TextGenerationSettings,
    },
    TextGenerationWithoutModel {
        chat_template_source: ChatTemplateSource,
    },
}

impl AgentApplicableState {
    #[must_use]
    pub fn chat_template_override(&self) -> Option<ChatTemplate> {
        match self {
            Self::TextGeneration {
                text_generation_settings:
                    TextGenerationSettings {
                        chat_template_source: ChatTemplateSource::Override(chat_template),
                        ..
                    },
                ..
            }
            | Self::TextGenerationWithoutModel {
                chat_template_source: ChatTemplateSource::Override(chat_template),
            } => Some(chat_template.clone()),
            Self::Decision { .. }
            | Self::Embeddings { .. }
            | Self::NotConfigured
            | Self::TextGeneration { .. }
            | Self::TextGenerationWithoutModel { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::chat_template::ChatTemplate;
    use paddler_messaging::chat_template_source::ChatTemplateSource;

    use super::AgentApplicableState;

    #[test]
    fn a_text_generation_state_without_a_model_overrides_its_chat_template() {
        let chat_template = ChatTemplate {
            content: "{{ messages }}".to_owned(),
        };

        assert_eq!(
            AgentApplicableState::TextGenerationWithoutModel {
                chat_template_source: ChatTemplateSource::Override(chat_template.clone()),
            }
            .chat_template_override(),
            Some(chat_template)
        );
    }
}
