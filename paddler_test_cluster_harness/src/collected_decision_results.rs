use paddler_messaging::decision_answer::DecisionAnswer;
use paddler_messaging::decision_result::DecisionResult;

pub struct CollectedDecisionResults {
    pub answers: Vec<DecisionAnswer>,
    pub terminal_result: DecisionResult,
}
