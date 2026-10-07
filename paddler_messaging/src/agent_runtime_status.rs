use serde::Deserialize;
use serde::Serialize;

use crate::inference_mode::InferenceMode;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum AgentRuntimeStatus {
    #[default]
    Idle,
    Serving {
        inference_mode: InferenceMode,
        slots_total: u64,
    },
}

impl AgentRuntimeStatus {
    #[must_use]
    pub const fn slots_total(&self) -> u64 {
        match self {
            Self::Idle => 0,
            Self::Serving { slots_total, .. } => *slots_total,
        }
    }

    #[must_use]
    pub fn serves(&self, served_inference_mode: InferenceMode) -> bool {
        match self {
            Self::Idle => false,
            Self::Serving { inference_mode, .. } => *inference_mode == served_inference_mode,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AgentRuntimeStatus;
    use crate::inference_mode::InferenceMode;

    const SERVING_EMBEDDINGS: AgentRuntimeStatus = AgentRuntimeStatus::Serving {
        inference_mode: InferenceMode::Embeddings,
        slots_total: 3,
    };

    #[test]
    fn an_idle_agent_has_no_slots() {
        assert_eq!(AgentRuntimeStatus::Idle.slots_total(), 0);
    }

    #[test]
    fn a_serving_agent_has_its_slots() {
        assert_eq!(SERVING_EMBEDDINGS.slots_total(), 3);
    }

    #[test]
    fn an_idle_agent_serves_no_mode() {
        assert!(!AgentRuntimeStatus::Idle.serves(InferenceMode::Embeddings));
    }

    #[test]
    fn a_serving_agent_serves_only_its_own_mode() {
        assert!(SERVING_EMBEDDINGS.serves(InferenceMode::Embeddings));
        assert!(!SERVING_EMBEDDINGS.serves(InferenceMode::TextGeneration));
    }
}
