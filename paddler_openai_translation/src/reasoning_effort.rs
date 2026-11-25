use serde::Deserialize;

use paddler_inference_parameters::model_default_or::ModelDefaultOr;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    None,
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
}

impl ReasoningEffort {
    #[must_use]
    pub const fn thinking(self) -> ModelDefaultOr<bool> {
        match self {
            Self::None => ModelDefaultOr::Explicit(false),
            Self::Minimal | Self::Low | Self::Medium | Self::High | Self::Xhigh => {
                ModelDefaultOr::ModelDefault
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use paddler_inference_parameters::model_default_or::ModelDefaultOr;

    use super::ReasoningEffort;

    #[test]
    fn only_no_effort_turns_thinking_off() {
        assert_eq!(
            ReasoningEffort::None.thinking(),
            ModelDefaultOr::Explicit(false)
        );
        assert_eq!(
            ReasoningEffort::Minimal.thinking(),
            ModelDefaultOr::ModelDefault
        );
    }
}
