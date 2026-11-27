use std::num::NonZeroU16;

use crate::agent_issue_params::slots_insufficient_for_decisions_params::SlotsInsufficientForDecisionsParams;
use crate::decision_slot_demand::DecisionSlotDemand;

pub enum DecisionSlotAllotment {
    Allotted(NonZeroU16),
    Insufficient(SlotsInsufficientForDecisionsParams),
}

impl DecisionSlotAllotment {
    #[must_use]
    pub fn of_desired_slots(desired_slots: u16, shares_prompt_prefix: bool) -> Self {
        let required_slots = DecisionSlotDemand::SeveralQuestions.slots(shares_prompt_prefix);

        NonZeroU16::new(desired_slots)
            .filter(|decision_slots| *decision_slots >= required_slots)
            .map_or_else(
                || {
                    Self::Insufficient(SlotsInsufficientForDecisionsParams {
                        desired_slots,
                        required_slots: required_slots.get(),
                    })
                },
                Self::Allotted,
            )
    }
}
