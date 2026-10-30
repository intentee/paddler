use paddler_agent_runtime::sequence_id_guard::SequenceIdGuard;

use crate::question_lane::QuestionLane;

pub struct DecisionAdmission {
    pub question_lane: QuestionLane,
    pub state_sequence: SequenceIdGuard,
}
