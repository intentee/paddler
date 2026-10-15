use serde::Deserialize;

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
    pub const fn enables_thinking(self) -> bool {
        match self {
            Self::None => false,
            Self::Minimal | Self::Low | Self::Medium | Self::High | Self::Xhigh => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ReasoningEffort;

    #[test]
    fn minimal_effort_still_enables_thinking() {
        assert!(ReasoningEffort::Minimal.enables_thinking());
    }
}
