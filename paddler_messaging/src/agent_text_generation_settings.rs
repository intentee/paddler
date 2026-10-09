use serde::Deserialize;
use serde::Serialize;

use paddler_inference_parameters::sampling_parameters::SamplingParameters;

use crate::balancer_text_generation_settings::BalancerTextGenerationSettings;
use crate::chat_template_source::ChatTemplateSource;
use crate::multimodal_settings::MultimodalSettings;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgentTextGenerationSettings {
    pub chat_template_source: ChatTemplateSource,
    pub multimodal: MultimodalSettings,
    pub sampling_parameters: SamplingParameters,
}

impl From<BalancerTextGenerationSettings> for AgentTextGenerationSettings {
    fn from(balancer_text_generation_settings: BalancerTextGenerationSettings) -> Self {
        Self {
            chat_template_source: balancer_text_generation_settings.chat_template_source(),
            multimodal: balancer_text_generation_settings.multimodal,
            sampling_parameters: balancer_text_generation_settings.sampling_parameters,
        }
    }
}

#[cfg(test)]
mod tests {
    use paddler_inference_parameters::sampling_parameters::SamplingParameters;

    use super::AgentTextGenerationSettings;
    use crate::balancer_text_generation_settings::BalancerTextGenerationSettings;
    use crate::chat_template::ChatTemplate;
    use crate::chat_template_source::ChatTemplateSource;

    #[test]
    fn text_generation_settings_reach_the_agent_with_the_resolved_chat_template() {
        let chat_template = ChatTemplate {
            content: "{{ messages }}".to_owned(),
        };

        assert_eq!(
            AgentTextGenerationSettings::from(BalancerTextGenerationSettings {
                chat_template_override: Some(chat_template.clone()),
                sampling_parameters: SamplingParameters::deterministic(),
                use_chat_template_override: true,
                ..BalancerTextGenerationSettings::default()
            }),
            AgentTextGenerationSettings {
                chat_template_source: ChatTemplateSource::Override(chat_template),
                sampling_parameters: SamplingParameters::deterministic(),
                ..AgentTextGenerationSettings::default()
            }
        );
    }
}
