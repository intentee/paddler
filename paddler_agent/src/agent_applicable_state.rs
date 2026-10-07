use std::path::PathBuf;

use paddler_agent_text_generation::text_generation_settings::TextGenerationSettings;
use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
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
        embedding_parameters: EmbeddingParameters,
        model_path: PathBuf,
        model_runtime_parameters: ModelRuntimeParameters,
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
