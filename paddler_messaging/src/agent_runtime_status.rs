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
    pub fn slots_serving(&self, requested_inference_mode: InferenceMode) -> u64 {
        match self {
            Self::Serving {
                inference_mode,
                slots_total,
            } if *inference_mode == requested_inference_mode => *slots_total,
            Self::Idle | Self::Serving { .. } => 0,
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
    fn a_serving_agent_offers_its_slots_to_the_inference_mode_it_serves() {
        assert_eq!(
            SERVING_EMBEDDINGS.slots_serving(InferenceMode::Embeddings),
            3
        );
    }

    #[test]
    fn a_serving_agent_offers_no_slots_to_another_inference_mode() {
        assert_eq!(
            SERVING_EMBEDDINGS.slots_serving(InferenceMode::TextGeneration),
            0
        );
    }
}
