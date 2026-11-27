use serde_json::Error as JsonError;
use serde_json::from_str;

use llama_cpp_bindings_types::decision_answer::DecisionAnswer;

use paddler_messaging::answered_decision_question::AnsweredDecisionQuestion;
use paddler_messaging::decision_summary::DecisionSummary;
use paddler_typesafe_translation::system_one_request::SystemOneRequest;
use paddler_typesafe_translation::translated_system_one_request::TranslatedSystemOneRequest;
use paddler_typesafe_translation::typesafe_translation_error::TypeSafeTranslationError;

const TWO_NOUL_QUESTIONS_REQUEST: &str = r#"{"model": "test", "questions": {"first": {"type": "noul"}, "second": {"type": "noul"}}, "state": "state"}"#;

const SUMMARY: DecisionSummary = DecisionSummary {
    input_tokens: 1,
    processing_milliseconds: 1,
};

/// # Errors
///
/// Returns the [`JsonError`] of a request with two noul questions that does not parse.
pub fn refusal_of_a_lone_answer_to_two_noul_questions(
    id: &str,
) -> Result<Option<TypeSafeTranslationError>, JsonError> {
    from_str::<SystemOneRequest>(TWO_NOUL_QUESTIONS_REQUEST).map(|system_one_request| {
        system_one_request
            .translate()
            .and_then(
                |TranslatedSystemOneRequest {
                     response_layout, ..
                 }| {
                    response_layout.respond(
                        vec![AnsweredDecisionQuestion {
                            answer: DecisionAnswer::Noul { noul: 0.5 },
                            id: id.to_owned(),
                        }],
                        &SUMMARY,
                    )
                },
            )
            .err()
    })
}
