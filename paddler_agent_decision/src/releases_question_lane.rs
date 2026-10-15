use paddler_agent_runtime::sequence_id::SequenceId;

use crate::decision_decoder::DecisionDecoder;
use crate::decision_error::DecisionError;

pub trait ReleasesQuestionLane: Send {
    fn release_question_lane(
        &self,
        decoder: &mut DecisionDecoder,
        question_lane: SequenceId,
    ) -> Result<(), DecisionError>;
}
