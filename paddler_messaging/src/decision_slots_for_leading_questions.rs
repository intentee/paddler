use std::num::NonZeroU16;

use crate::decision_slots_minimum::DECISION_SLOTS_MINIMUM;

#[must_use]
pub const fn decision_slots_for_leading_questions(leading_question_count: usize) -> NonZeroU16 {
    if leading_question_count == 0 {
        NonZeroU16::MIN
    } else {
        DECISION_SLOTS_MINIMUM
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU16;

    use super::decision_slots_for_leading_questions;
    use crate::decision_slots_minimum::DECISION_SLOTS_MINIMUM;

    #[test]
    fn a_decision_without_leading_questions_needs_only_its_state_slot() {
        assert_eq!(decision_slots_for_leading_questions(0), NonZeroU16::MIN);
    }

    #[test]
    fn a_decision_with_leading_questions_needs_its_state_and_question_lane_slots() {
        assert_eq!(
            decision_slots_for_leading_questions(3),
            DECISION_SLOTS_MINIMUM
        );
    }
}
