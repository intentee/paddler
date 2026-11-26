use std::num::NonZeroU16;

use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
use paddler_messaging::decision_slot_demand::DecisionSlotDemand;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SlotDemand {
    Decision(DecisionSlotDemand),
    Exactly(NonZeroU16),
    UpTo(u16),
}

impl SlotDemand {
    #[must_use]
    pub const fn minimum_slots_on(self, agent_runtime_status: &AgentRuntimeStatus) -> u16 {
        match self {
            Self::Decision(decision_slot_demand) => {
                agent_runtime_status.slots_a_decision_takes(decision_slot_demand)
            }
            Self::Exactly(slots) => slots.get(),
            Self::UpTo(_slots) => 1,
        }
    }

    #[must_use]
    pub fn reservable_on(
        self,
        agent_runtime_status: &AgentRuntimeStatus,
        free_slots: u64,
    ) -> Option<NonZeroU16> {
        match self {
            Self::Decision(_) | Self::Exactly(_) => {
                NonZeroU16::new(self.minimum_slots_on(agent_runtime_status))
                    .filter(|slots| free_slots >= u64::from(slots.get()))
            }
            Self::UpTo(slots) => NonZeroU16::new(
                u16::try_from(free_slots).map_or(slots, |free_slots| free_slots.min(slots)),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU16;

    use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
    use paddler_messaging::constants::DECISION_SLOTS_WITH_A_QUESTION_LANE;
    use paddler_messaging::decision_slot_demand::DecisionSlotDemand;
    use paddler_model_capabilities_fixture::chat_model_capabilities::chat_model_capabilities;
    use paddler_model_capabilities_fixture::decision_model_capabilities::decision_model_capabilities;

    use super::SlotDemand;

    const SERVING_A_CHAT_MODEL: AgentRuntimeStatus = AgentRuntimeStatus::Serving {
        capabilities: chat_model_capabilities(),
        slots_total: 8,
    };
    const SERVING_A_DECISION_MODEL: AgentRuntimeStatus = AgentRuntimeStatus::Serving {
        capabilities: decision_model_capabilities(),
        slots_total: 8,
    };
    const TWO_SLOTS: NonZeroU16 = NonZeroU16::new(2).unwrap();

    #[test]
    fn an_exact_demand_reserves_its_slots_when_they_are_free() {
        assert_eq!(
            SlotDemand::Exactly(TWO_SLOTS).reservable_on(&SERVING_A_CHAT_MODEL, 3),
            Some(TWO_SLOTS)
        );
    }

    #[test]
    fn an_exact_demand_reserves_nothing_while_fewer_slots_are_free() {
        assert_eq!(
            SlotDemand::Exactly(TWO_SLOTS).reservable_on(&SERVING_A_CHAT_MODEL, 1),
            None
        );
    }

    #[test]
    fn a_flexible_demand_reserves_every_free_slot_it_can_use() {
        assert_eq!(
            SlotDemand::UpTo(5).reservable_on(&SERVING_A_CHAT_MODEL, 2),
            Some(TWO_SLOTS)
        );
        assert_eq!(
            SlotDemand::UpTo(2).reservable_on(&SERVING_A_CHAT_MODEL, 5),
            Some(TWO_SLOTS)
        );
    }

    #[test]
    fn a_flexible_demand_reserves_nothing_while_no_slot_is_free() {
        assert_eq!(
            SlotDemand::UpTo(5).reservable_on(&SERVING_A_CHAT_MODEL, 0),
            None
        );
    }

    #[test]
    fn a_decision_reserves_the_slots_it_takes_on_the_model_of_the_agent() {
        assert_eq!(
            SlotDemand::Decision(DecisionSlotDemand::SeveralQuestions)
                .reservable_on(&SERVING_A_DECISION_MODEL, 8),
            Some(DECISION_SLOTS_WITH_A_QUESTION_LANE)
        );
    }

    #[test]
    fn a_decision_reserves_nothing_from_an_agent_serving_no_decisions() {
        assert_eq!(
            SlotDemand::Decision(DecisionSlotDemand::OneQuestion)
                .reservable_on(&SERVING_A_CHAT_MODEL, 8),
            None
        );
    }
}
