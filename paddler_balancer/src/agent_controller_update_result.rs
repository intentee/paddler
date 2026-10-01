#[derive(Debug, Eq, PartialEq)]
pub enum AgentControllerUpdateResult {
    NoMeaningfulChanges,
    Updated,
}
