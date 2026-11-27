use paddler_typesafe_translation::typesafe_translation_error::TypeSafeTranslationError;
use paddler_typesafe_translation_tests::refusal_of_a_lone_answer_to_two_noul_questions::refusal_of_a_lone_answer_to_two_noul_questions;

#[test]
fn a_question_left_unanswered_is_refused() {
    assert!(matches!(
        refusal_of_a_lone_answer_to_two_noul_questions("first")
            .expect("the request with two noul questions must parse"),
        Some(TypeSafeTranslationError::AnswerMissing { question_id }) if question_id == "second"
    ));
}
