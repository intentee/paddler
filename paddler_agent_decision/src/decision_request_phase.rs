#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecisionRequestPhase {
    AnsweringQuestion { next_offset: usize },
    IngestingState { next_offset: usize },
}
