use paddler_agent_runtime::sequence_id::SequenceId;

use crate::decision_decoder::DecisionDecoder;
use crate::decision_error::DecisionError;
use crate::releases_question_lane::ReleasesQuestionLane;

pub struct LlamaContextQuestionLaneRelease;

impl ReleasesQuestionLane for LlamaContextQuestionLaneRelease {
    fn release_question_lane(
        &self,
        decoder: &mut DecisionDecoder,
        question_lane: SequenceId,
    ) -> Result<(), DecisionError> {
        decoder.remove_sequence(question_lane)
    }
}
