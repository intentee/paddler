use paddler_agent_runtime::sequence_id_guard::SequenceIdGuard;

pub enum QuestionLane {
    Released,
    Reserved(SequenceIdGuard),
}
