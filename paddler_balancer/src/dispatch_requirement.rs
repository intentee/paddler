use std::num::NonZeroU16;

use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
use paddler_messaging::inference_capability::InferenceCapability;

use crate::slot_demand::SlotDemand;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DispatchRequirement {
    pub inference_capability: InferenceCapability,
    pub slot_demand: SlotDemand,
}

impl DispatchRequirement {
    #[must_use]
    pub fn reservable_slots(
        &self,
        agent_runtime_status: &AgentRuntimeStatus,
        slots_unavailable: u64,
    ) -> Option<NonZeroU16> {
        self.slot_demand.reservable_from(
            agent_runtime_status
                .slots_serving(self.inference_capability)
                .saturating_sub(slots_unavailable),
        )
    }

    #[must_use]
    pub fn slots_held_while_waiting(&self, agent_runtime_status: &AgentRuntimeStatus) -> u64 {
        let minimum_slots = u64::from(self.slot_demand.minimum_slots());

        if agent_runtime_status.slots_serving(self.inference_capability) >= minimum_slots {
            minimum_slots
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU16;

    use paddler_messaging::agent_decisions_serving::AgentDecisionsServing;
    use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
    use paddler_messaging::decision_question::DecisionQuestion;
    use paddler_messaging::inference_capability::InferenceCapability;
    use paddler_messaging::request_params::decide_params::DecideParams;

    use super::DispatchRequirement;
    use crate::agent_streaming_request::AgentStreamingRequest as _;
    use crate::slot_demand::SlotDemand;

    const SERVING_FOUR_SLOTS: AgentRuntimeStatus = AgentRuntimeStatus::Serving {
        decisions: AgentDecisionsServing::NotServed,
        slots_total: 4,
    };

    fn decision_question(id: &str) -> DecisionQuestion {
        DecisionQuestion {
            id: id.to_owned(),
            instructions: "Pick one".to_owned(),
            options: vec!["yes".to_owned(), "no".to_owned()],
        }
    }

    fn decision_with_a_leading_question() -> DispatchRequirement {
        DecideParams {
            last_question: decision_question("last"),
            leading_questions: vec![decision_question("leading")],
            state: "state".to_owned(),
        }
        .dispatch_requirement()
    }

    fn requirement(inference_capability: InferenceCapability) -> DispatchRequirement {
        DispatchRequirement {
            inference_capability,
            slot_demand: SlotDemand::UpTo(8),
        }
    }

    #[test]
    fn reserves_the_slots_left_after_the_ones_in_use() {
        assert_eq!(
            requirement(InferenceCapability::Embeddings).reservable_slots(&SERVING_FOUR_SLOTS, 1),
            NonZeroU16::new(3)
        );
    }

    #[test]
    fn reserves_nothing_from_an_agent_using_more_slots_than_it_serves() {
        assert_eq!(
            requirement(InferenceCapability::Embeddings).reservable_slots(&SERVING_FOUR_SLOTS, 5),
            None
        );
    }

    #[test]
    fn reserves_nothing_for_a_capability_the_agent_does_not_serve() {
        assert_eq!(
            requirement(InferenceCapability::Decisions).reservable_slots(&SERVING_FOUR_SLOTS, 0),
            None
        );
    }

    #[test]
    fn a_waiting_requirement_holds_its_minimum_slots_on_an_agent_that_serves_it() {
        assert_eq!(
            decision_with_a_leading_question().slots_held_while_waiting(
                &AgentRuntimeStatus::Serving {
                    decisions: AgentDecisionsServing::Served,
                    slots_total: 4,
                }
            ),
            2
        );
    }

    #[test]
    fn a_waiting_requirement_holds_no_slots_on_an_agent_that_cannot_serve_it() {
        assert_eq!(
            decision_with_a_leading_question().slots_held_while_waiting(&SERVING_FOUR_SLOTS),
            0
        );
    }
}
