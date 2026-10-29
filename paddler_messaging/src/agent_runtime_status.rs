use serde::Deserialize;
use serde::Serialize;

use crate::agent_decisions_serving::AgentDecisionsServing;
use crate::inference_capability::InferenceCapability;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum AgentRuntimeStatus {
    #[default]
    Idle,
    Serving {
        decisions: AgentDecisionsServing,
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
    pub const fn serves(&self, inference_capability: InferenceCapability) -> bool {
        match self {
            Self::Idle => false,
            Self::Serving { decisions, .. } => decisions.serves(inference_capability),
        }
    }

    #[must_use]
    pub const fn slots_serving(&self, inference_capability: InferenceCapability) -> u64 {
        if self.serves(inference_capability) {
            self.slots_total()
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AgentRuntimeStatus;
    use crate::agent_decisions_serving::AgentDecisionsServing;
    use crate::inference_capability::InferenceCapability;

    const SERVING_WITHOUT_DECISIONS: AgentRuntimeStatus = AgentRuntimeStatus::Serving {
        decisions: AgentDecisionsServing::NotServed,
        slots_total: 3,
    };

    #[test]
    fn an_idle_agent_has_no_slots() {
        assert_eq!(AgentRuntimeStatus::Idle.slots_total(), 0);
    }

    #[test]
    fn a_serving_agent_has_its_slots() {
        assert_eq!(SERVING_WITHOUT_DECISIONS.slots_total(), 3);
    }

    #[test]
    fn a_serving_agent_offers_its_slots_to_every_capability_it_serves() {
        assert_eq!(
            SERVING_WITHOUT_DECISIONS.slots_serving(InferenceCapability::Embeddings),
            3
        );
        assert_eq!(
            SERVING_WITHOUT_DECISIONS.slots_serving(InferenceCapability::TextGeneration),
            3
        );
    }

    #[test]
    fn a_serving_agent_offers_no_slots_to_a_capability_it_does_not_serve() {
        assert_eq!(
            SERVING_WITHOUT_DECISIONS.slots_serving(InferenceCapability::Decisions),
            0
        );
    }

    #[test]
    fn an_idle_agent_offers_no_slots_to_any_capability() {
        assert_eq!(
            AgentRuntimeStatus::Idle.slots_serving(InferenceCapability::TextGeneration),
            0
        );
    }
}
