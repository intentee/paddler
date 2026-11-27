use paddler_messaging::answered_decision_question::AnsweredDecisionQuestion;
use paddler_messaging::decision_result::DecisionResult;

#[derive(Debug)]
pub struct CollectedDecisionResults {
    pub answers: Vec<AnsweredDecisionQuestion>,
    pub terminal_result: DecisionResult,
}
