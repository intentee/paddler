use paddler_agent_text_generation::text_generation_settings::TextGenerationSettings;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::chat_template_source::ChatTemplateSource;

use crate::inference_capabilities::InferenceCapabilities;
use crate::model_serving_settings::ModelServingSettings;

#[derive(Clone, Debug, Default, PartialEq)]
pub enum AgentApplicableState {
    #[default]
    NotConfigured,
    WithModel(ModelServingSettings),
    WithoutModel {
        chat_template_source: ChatTemplateSource,
    },
}

impl AgentApplicableState {
    #[must_use]
    pub fn chat_template_override(&self) -> Option<ChatTemplate> {
        match self {
            Self::WithModel(ModelServingSettings {
                capabilities:
                    InferenceCapabilities {
                        text_generation:
                            TextGenerationSettings {
                                chat_template_source: ChatTemplateSource::Override(chat_template),
                                ..
                            },
                        ..
                    },
                ..
            })
            | Self::WithoutModel {
                chat_template_source: ChatTemplateSource::Override(chat_template),
            } => Some(chat_template.clone()),
            Self::NotConfigured | Self::WithModel(_) | Self::WithoutModel { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::chat_template::ChatTemplate;
    use paddler_messaging::chat_template_source::ChatTemplateSource;

    use super::AgentApplicableState;

    #[test]
    fn a_state_without_a_model_overrides_its_chat_template() {
        let chat_template = ChatTemplate {
            content: "{{ messages }}".to_owned(),
        };

        assert_eq!(
            AgentApplicableState::WithoutModel {
                chat_template_source: ChatTemplateSource::Override(chat_template.clone()),
            }
            .chat_template_override(),
            Some(chat_template)
        );
    }
}
