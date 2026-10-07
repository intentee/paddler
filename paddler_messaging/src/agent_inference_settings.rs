use serde::Deserialize;
use serde::Serialize;

use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;

use crate::agent_text_generation_settings::AgentTextGenerationSettings;
use crate::balancer_inference_settings::BalancerInferenceSettings;
use crate::decision_settings::DecisionSettings;
use crate::inference_mode::InferenceMode;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum AgentInferenceSettings {
    Decision(DecisionSettings),
    Embeddings(EmbeddingParameters),
    TextGeneration(AgentTextGenerationSettings),
}

impl AgentInferenceSettings {
    #[must_use]
    pub const fn inference_mode(&self) -> InferenceMode {
        match self {
            Self::Decision(_) => InferenceMode::Decision,
            Self::Embeddings(_) => InferenceMode::Embeddings,
            Self::TextGeneration(_) => InferenceMode::TextGeneration,
        }
    }
}

impl From<BalancerInferenceSettings> for AgentInferenceSettings {
    fn from(balancer_inference_settings: BalancerInferenceSettings) -> Self {
        match balancer_inference_settings {
            BalancerInferenceSettings::Decision(decision_settings) => {
                Self::Decision(decision_settings)
            }
            BalancerInferenceSettings::Embeddings(embedding_parameters) => {
                Self::Embeddings(embedding_parameters)
            }
            BalancerInferenceSettings::TextGeneration(text_generation_settings) => {
                Self::TextGeneration(AgentTextGenerationSettings {
                    chat_template_source: text_generation_settings.chat_template_source(),
                    multimodal: text_generation_settings.multimodal.clone(),
                    sampling_parameters: text_generation_settings.sampling_parameters,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
    use paddler_inference_parameters::sampling_parameters::SamplingParameters;

    use super::AgentInferenceSettings;
    use crate::agent_text_generation_settings::AgentTextGenerationSettings;
    use crate::balancer_inference_settings::BalancerInferenceSettings;
    use crate::balancer_text_generation_settings::BalancerTextGenerationSettings;
    use crate::chat_template::ChatTemplate;
    use crate::chat_template_source::ChatTemplateSource;
    use crate::inference_mode::InferenceMode;

    #[test]
    fn embedding_parameters_reach_the_agent_unchanged() {
        assert_eq!(
            AgentInferenceSettings::from(BalancerInferenceSettings::Embeddings(
                EmbeddingParameters::default()
            )),
            AgentInferenceSettings::Embeddings(EmbeddingParameters::default())
        );
    }

    #[test]
    fn text_generation_settings_reach_the_agent_with_the_resolved_chat_template() {
        let chat_template = ChatTemplate {
            content: "{{ messages }}".to_owned(),
        };

        assert_eq!(
            AgentInferenceSettings::from(BalancerInferenceSettings::TextGeneration(
                BalancerTextGenerationSettings {
                    chat_template_override: Some(chat_template.clone()),
                    sampling_parameters: SamplingParameters::deterministic(),
                    use_chat_template_override: true,
                    ..BalancerTextGenerationSettings::default()
                }
            )),
            AgentInferenceSettings::TextGeneration(AgentTextGenerationSettings {
                chat_template_source: ChatTemplateSource::Override(chat_template),
                sampling_parameters: SamplingParameters::deterministic(),
                ..AgentTextGenerationSettings::default()
            })
        );
    }

    #[test]
    fn agent_settings_serve_the_mode_of_the_balancer_settings() {
        for inference_mode in [
            InferenceMode::Decision,
            InferenceMode::Embeddings,
            InferenceMode::TextGeneration,
        ] {
            assert_eq!(
                AgentInferenceSettings::from(BalancerInferenceSettings::unconfigured(
                    inference_mode
                ))
                .inference_mode(),
                inference_mode
            );
        }
    }
}
