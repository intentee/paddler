use llama_cpp_bindings_types::decision_capabilities::DecisionCapabilities;
use llama_cpp_bindings_types::decisions_capability::DecisionsCapability;
use llama_cpp_bindings_types::model_capabilities::ModelCapabilities;
use serde::Deserialize;
use serde::Serialize;

use crate::decision_slot_demand::DecisionSlotDemand;
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
    pub const fn slots_a_decision_takes(&self, decision_slot_demand: DecisionSlotDemand) -> u16 {
        match self {
            Self::Serving {
                capabilities:
                    ModelCapabilities {
                        decisions:
                            DecisionsCapability::Supported(DecisionCapabilities {
                                shares_prompt_prefix,
                                ..
                            }),
                        ..
                    },
                ..
            } => decision_slot_demand.slots(*shares_prompt_prefix).get(),
            Self::Idle | Self::Serving { .. } => 0,
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
    use paddler_model_capabilities_fixture::decision_model_capabilities::decision_model_capabilities;

    use super::AgentRuntimeStatus;
    use crate::constants::DECISION_SLOTS_WITH_A_QUESTION_LANE;
    use crate::decision_slot_demand::DecisionSlotDemand;
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

    #[test]
    fn an_agent_serving_a_model_that_shares_prompt_prefixes_takes_a_question_lane_for_several_questions()
     {
        assert_eq!(
            AgentRuntimeStatus::Serving {
                capabilities: decision_model_capabilities(),
                slots_total: 3,
            }
            .slots_a_decision_takes(DecisionSlotDemand::SeveralQuestions),
            DECISION_SLOTS_WITH_A_QUESTION_LANE.get()
        );
    }

    #[test]
    fn an_agent_serving_no_decisions_takes_no_slots_for_a_decision() {
        assert_eq!(
            SERVING_A_CHAT_MODEL.slots_a_decision_takes(DecisionSlotDemand::SeveralQuestions),
            0
        );
    }
}
