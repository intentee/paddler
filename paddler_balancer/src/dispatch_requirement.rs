use std::num::NonZeroU16;

use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
use paddler_messaging::requested_capability::RequestedCapability;

use crate::slot_demand::SlotDemand;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DispatchRequirement {
    pub requested_capability: RequestedCapability,
    pub slot_demand: SlotDemand,
}

impl DispatchRequirement {
    #[must_use]
    pub fn reservable_slots(
        self,
        agent_runtime_status: &AgentRuntimeStatus,
        slots_unavailable: u64,
    ) -> Option<NonZeroU16> {
        self.slot_demand.reservable_on(
            agent_runtime_status,
            agent_runtime_status
                .slots_serving(self.requested_capability)
                .saturating_sub(slots_unavailable),
        )
    }

    #[must_use]
    pub fn slots_held_while_waiting(self, agent_runtime_status: &AgentRuntimeStatus) -> u64 {
        let minimum_slots = u64::from(self.slot_demand.minimum_slots_on(agent_runtime_status));

        if agent_runtime_status.slots_serving(self.requested_capability) >= minimum_slots {
            minimum_slots
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU16;

    use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
    use paddler_messaging::requested_capability::RequestedCapability;
    use paddler_model_capabilities_fixture::chat_model_capabilities::chat_model_capabilities;

    use super::DispatchRequirement;
    use crate::slot_demand::SlotDemand;

    const SERVING_A_CHAT_MODEL_ON_FOUR_SLOTS: AgentRuntimeStatus = AgentRuntimeStatus::Serving {
        capabilities: chat_model_capabilities(),
        slots_total: 4,
    };

    const fn requirement_of_exactly_one_slot(
        requested_capability: RequestedCapability,
    ) -> DispatchRequirement {
        DispatchRequirement {
            requested_capability,
            slot_demand: SlotDemand::Exactly(NonZeroU16::MIN),
        }
    }

    const fn requirement_of_up_to_eight_slots(
        requested_capability: RequestedCapability,
    ) -> DispatchRequirement {
        DispatchRequirement {
            requested_capability,
            slot_demand: SlotDemand::UpTo(8),
        }
    }

    #[test]
    fn reserves_the_slots_left_after_the_ones_in_use() {
        assert_eq!(
            requirement_of_up_to_eight_slots(RequestedCapability::Conversation)
                .reservable_slots(&SERVING_A_CHAT_MODEL_ON_FOUR_SLOTS, 1),
            NonZeroU16::new(3)
        );
    }

    #[test]
    fn reserves_nothing_from_an_agent_using_more_slots_than_it_serves() {
        assert_eq!(
            requirement_of_up_to_eight_slots(RequestedCapability::Conversation)
                .reservable_slots(&SERVING_A_CHAT_MODEL_ON_FOUR_SLOTS, 5),
            None
        );
    }

    #[test]
    fn reserves_nothing_for_a_capability_the_agent_does_not_serve() {
        assert_eq!(
            requirement_of_up_to_eight_slots(RequestedCapability::Decisions)
                .reservable_slots(&SERVING_A_CHAT_MODEL_ON_FOUR_SLOTS, 0),
            None
        );
    }

    #[test]
    fn a_waiting_requirement_holds_its_minimum_slots_on_an_agent_that_serves_it() {
        assert_eq!(
            requirement_of_exactly_one_slot(RequestedCapability::Conversation)
                .slots_held_while_waiting(&SERVING_A_CHAT_MODEL_ON_FOUR_SLOTS),
            1
        );
    }

    #[test]
    fn a_waiting_requirement_holds_no_slots_on_an_agent_that_cannot_serve_it() {
        assert_eq!(
            requirement_of_exactly_one_slot(RequestedCapability::Decisions)
                .slots_held_while_waiting(&SERVING_A_CHAT_MODEL_ON_FOUR_SLOTS),
            0
        );
    }
}
