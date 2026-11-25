use std::num::NonZeroU16;

use crate::agent_issue_params::slots_insufficient_for_decisions_params::SlotsInsufficientForDecisionsParams;
use crate::constants::DECISION_SLOTS_MINIMUM;

pub enum DecisionSlotAllotment {
    Allotted(NonZeroU16),
    Insufficient(SlotsInsufficientForDecisionsParams),
}

impl DecisionSlotAllotment {
    #[must_use]
    pub fn of_desired_slots(desired_slots: u16) -> Self {
        NonZeroU16::new(desired_slots)
            .filter(|decision_slots| *decision_slots >= DECISION_SLOTS_MINIMUM)
            .map_or_else(
                || {
                    Self::Insufficient(SlotsInsufficientForDecisionsParams {
                        desired_slots,
                        required_slots: DECISION_SLOTS_MINIMUM.get(),
                    })
                },
                Self::Allotted,
            )
    }
}
