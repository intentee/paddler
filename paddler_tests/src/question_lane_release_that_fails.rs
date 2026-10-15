use llama_cpp_bindings::error::ClearKvCacheSeqError;

use paddler_agent_decision::decision_decoder::DecisionDecoder;
use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_decision::releases_question_lane::ReleasesQuestionLane;
use paddler_agent_runtime::sequence_id::SequenceId;

pub struct QuestionLaneReleaseThatFails;

impl ReleasesQuestionLane for QuestionLaneReleaseThatFails {
    fn release_question_lane(
        &self,
        _decoder: &mut DecisionDecoder,
        question_lane: SequenceId,
    ) -> Result<(), DecisionError> {
        Err(DecisionError::KvCacheRemovalFailed(
            ClearKvCacheSeqError::PartialSequenceNotRemoved {
                seq_id: question_lane.llama_sequence_id(),
                p0: -1,
                p1: -1,
            },
        ))
    }
}
