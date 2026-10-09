use serde::Deserialize;
use serde::Serialize;

use paddler_inference_parameters::sampling_parameters::SamplingParameters;

use crate::chat_template::ChatTemplate;
use crate::chat_template_source::ChatTemplateSource;
use crate::multimodal_settings::MultimodalSettings;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BalancerTextGenerationSettings {
    pub chat_template_override: Option<ChatTemplate>,
    pub multimodal: MultimodalSettings,
    pub sampling_parameters: SamplingParameters,
    pub use_chat_template_override: bool,
}

impl BalancerTextGenerationSettings {
    #[must_use]
    pub fn chat_template_source(&self) -> ChatTemplateSource {
        match (
            self.use_chat_template_override,
            &self.chat_template_override,
        ) {
            (true, Some(chat_template)) => ChatTemplateSource::Override(chat_template.clone()),
            (true, None) | (false, _) => ChatTemplateSource::EmbeddedInModel,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BalancerTextGenerationSettings;
    use crate::chat_template::ChatTemplate;
    use crate::chat_template_source::ChatTemplateSource;

    fn chat_template() -> ChatTemplate {
        ChatTemplate {
            content: "{{ messages }}".to_owned(),
        }
    }

    #[test]
    fn an_enabled_override_replaces_the_embedded_template() {
        assert_eq!(
            BalancerTextGenerationSettings {
                chat_template_override: Some(chat_template()),
                use_chat_template_override: true,
                ..BalancerTextGenerationSettings::default()
            }
            .chat_template_source(),
            ChatTemplateSource::Override(chat_template())
        );
    }

    #[test]
    fn a_stored_override_that_is_not_used_keeps_the_embedded_template() {
        assert_eq!(
            BalancerTextGenerationSettings {
                chat_template_override: Some(chat_template()),
                use_chat_template_override: false,
                ..BalancerTextGenerationSettings::default()
            }
            .chat_template_source(),
            ChatTemplateSource::EmbeddedInModel
        );
    }

    #[test]
    fn using_an_override_that_was_never_written_keeps_the_embedded_template() {
        assert_eq!(
            BalancerTextGenerationSettings {
                chat_template_override: None,
                use_chat_template_override: true,
                ..BalancerTextGenerationSettings::default()
            }
            .chat_template_source(),
            ChatTemplateSource::EmbeddedInModel
        );
    }
}
