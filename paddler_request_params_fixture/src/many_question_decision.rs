use llama_cpp_bindings_types::decision_request::DecisionRequest;

use crate::sample_decision::sample_decision;

#[must_use]
pub fn many_question_decision(question_count: usize) -> DecisionRequest {
    let DecisionRequest {
        images,
        questions,
        state,
    } = sample_decision();

    DecisionRequest {
        images,
        questions: questions
            .iter()
            .cycle()
            .take(question_count)
            .enumerate()
            .map(|(question_index, (question_id, question))| {
                (format!("{question_id}-{question_index}"), question.clone())
            })
            .collect(),
        state,
    }
}
