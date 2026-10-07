use paddler_balancer::compatibility::compatibility_service_configuration::CompatibilityServiceConfiguration;
use paddler_messaging::inference_mode::InferenceMode;

#[derive(Clone)]
pub enum BalancerServingMode {
    Decision {
        typesafe_service_configuration: Option<CompatibilityServiceConfiguration>,
    },
    Embeddings,
    TextGeneration {
        openai_service_configuration: Option<CompatibilityServiceConfiguration>,
    },
}

impl BalancerServingMode {
    #[must_use]
    pub const fn inference_mode(&self) -> InferenceMode {
        match self {
            Self::Decision { .. } => InferenceMode::Decision,
            Self::Embeddings => InferenceMode::Embeddings,
            Self::TextGeneration { .. } => InferenceMode::TextGeneration,
        }
    }

    #[must_use]
    pub fn openai_service_configuration(&self) -> Option<CompatibilityServiceConfiguration> {
        match self {
            Self::Decision { .. } | Self::Embeddings => None,
            Self::TextGeneration {
                openai_service_configuration,
            } => openai_service_configuration.clone(),
        }
    }

    #[must_use]
    pub fn typesafe_service_configuration(&self) -> Option<CompatibilityServiceConfiguration> {
        match self {
            Self::Decision {
                typesafe_service_configuration,
            } => typesafe_service_configuration.clone(),
            Self::Embeddings | Self::TextGeneration { .. } => None,
        }
    }
}
