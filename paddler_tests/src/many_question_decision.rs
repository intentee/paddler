use paddler_messaging::decision_question::DecisionQuestion;
use paddler_messaging::request_params::decide_params::raw_decide_params::RawDecideParams;

use crate::sample_decision::sample_decision;

#[must_use]
pub fn many_question_decision(question_count: usize) -> RawDecideParams {
    let RawDecideParams { questions, state } = sample_decision();

    RawDecideParams {
        questions: questions
            .iter()
            .cycle()
            .take(question_count)
            .enumerate()
            .map(|(question_index, question)| DecisionQuestion {
                id: format!("{}-{question_index}", question.id),
                ..question.clone()
            })
            .collect(),
        state,
    }
}
