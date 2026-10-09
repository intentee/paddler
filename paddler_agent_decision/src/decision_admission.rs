use paddler_agent_runtime::sequence_id_guard::SequenceIdGuard;

use crate::promised_decision_cells::PromisedDecisionCells;
use crate::question_lane::QuestionLane;

pub struct DecisionAdmission {
    pub promised_cells: PromisedDecisionCells,
    pub question_lane: QuestionLane,
    pub state_sequence: SequenceIdGuard,
}
