use serde_json::Error as JsonError;

#[derive(Debug, thiserror::Error)]
pub enum TypeSafeTranslationError {
    #[error(
        "question {question_id} expected {expected} probabilities, the agent answered with {received}"
    )]
    AnswerProbabilitiesMismatch {
        expected: usize,
        question_id: String,
        received: usize,
    },

    #[error("the agent finished without answering question {question_id}")]
    AnswerMissing { question_id: String },

    #[error(
        "the agent answered question {received_question_id} where {expected_question_id} was due"
    )]
    AnswerOutOfOrder {
        expected_question_id: String,
        received_question_id: String,
    },

    #[error(
        "choice question {question_id} has {criteria_count} criteria, it needs between 1 and 255"
    )]
    ChoiceCriteriaOutOfRange {
        criteria_count: usize,
        question_id: String,
    },

    #[error("question {question_id} is malformed: {source}")]
    QuestionMalformed {
        question_id: String,
        #[source]
        source: JsonError,
    },

    #[error("a System One request needs at least one question")]
    QuestionsMissing,

    #[error("score question {question_id} has {criteria_count} levels, it needs between 1 and 255")]
    ScoreCriteriaOutOfRange {
        criteria_count: usize,
        question_id: String,
    },
}
