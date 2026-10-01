use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OpenAIReasoningEffort {
    None,
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
}

impl OpenAIReasoningEffort {
    #[must_use]
    pub const fn enables_thinking(self) -> bool {
        match self {
            Self::None => false,
            Self::Minimal | Self::Low | Self::Medium | Self::High | Self::Xhigh => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::OpenAIReasoningEffort;

    #[test]
    fn minimal_effort_still_enables_thinking() {
        assert!(OpenAIReasoningEffort::Minimal.enables_thinking());
    }
}
