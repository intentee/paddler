use std::num::NonZeroU16;

use crate::constants::DECISION_SLOTS_WITH_A_QUESTION_LANE;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecisionSlotDemand {
    OneQuestion,
    SeveralQuestions,
}

impl DecisionSlotDemand {
    #[must_use]
    pub const fn slots(self, shares_prompt_prefix: bool) -> NonZeroU16 {
        match self {
            Self::SeveralQuestions if shares_prompt_prefix => DECISION_SLOTS_WITH_A_QUESTION_LANE,
            Self::OneQuestion | Self::SeveralQuestions => NonZeroU16::MIN,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU16;

    use super::DecisionSlotDemand;
    use crate::constants::DECISION_SLOTS_WITH_A_QUESTION_LANE;

    #[test]
    fn several_questions_sharing_their_prompt_prefix_take_a_question_lane() {
        assert_eq!(
            DecisionSlotDemand::SeveralQuestions.slots(true),
            DECISION_SLOTS_WITH_A_QUESTION_LANE
        );
    }

    #[test]
    fn several_questions_sharing_no_prompt_prefix_take_one_slot() {
        assert_eq!(
            DecisionSlotDemand::SeveralQuestions.slots(false),
            NonZeroU16::MIN
        );
    }

    #[test]
    fn one_question_takes_one_slot_even_when_prompts_share_their_prefix() {
        assert_eq!(DecisionSlotDemand::OneQuestion.slots(true), NonZeroU16::MIN);
    }
}
