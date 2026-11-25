use llama_cpp_bindings_types::model_capabilities::ModelCapabilities;
use serde::Deserialize;
use serde::Serialize;

use crate::requested_capability::RequestedCapability;

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum AgentRuntimeStatus {
    #[default]
    Idle,
    Serving {
        capabilities: ModelCapabilities,
        slots_total: u64,
    },
}

impl AgentRuntimeStatus {
    #[must_use]
    pub const fn serves(&self, requested_capability: RequestedCapability) -> bool {
        match self {
            Self::Idle => false,
            Self::Serving { capabilities, .. } => {
                requested_capability.is_supported_by(capabilities)
            }
        }
    }

    #[must_use]
    pub const fn slots_serving(&self, requested_capability: RequestedCapability) -> u64 {
        if self.serves(requested_capability) {
            self.slots_total()
        } else {
            0
        }
    }

    #[must_use]
    pub const fn slots_total(&self) -> u64 {
        match self {
            Self::Idle => 0,
            Self::Serving { slots_total, .. } => *slots_total,
        }
    }
}

#[cfg(test)]
mod tests {
    use paddler_model_capabilities_fixture::chat_model_capabilities::chat_model_capabilities;

    use super::AgentRuntimeStatus;
    use crate::requested_capability::RequestedCapability;

    const SERVING_A_CHAT_MODEL: AgentRuntimeStatus = AgentRuntimeStatus::Serving {
        capabilities: chat_model_capabilities(),
        slots_total: 3,
    };

    #[test]
    fn a_serving_agent_offers_its_slots_to_what_its_model_supports() {
        assert_eq!(
            SERVING_A_CHAT_MODEL.slots_serving(RequestedCapability::Conversation),
            3
        );
    }

    #[test]
    fn a_serving_agent_offers_no_slots_to_what_its_model_does_not_support() {
        assert_eq!(
            SERVING_A_CHAT_MODEL.slots_serving(RequestedCapability::Decisions),
            0
        );
    }

    #[test]
    fn an_idle_agent_offers_no_slots_to_any_capability() {
        assert_eq!(
            AgentRuntimeStatus::Idle.slots_serving(RequestedCapability::RawTextGeneration),
            0
        );
    }
}
