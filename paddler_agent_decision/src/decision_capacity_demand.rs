#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecisionCapacityDemand {
    pub needs_question_lane: bool,
    pub required_cells: usize,
}
