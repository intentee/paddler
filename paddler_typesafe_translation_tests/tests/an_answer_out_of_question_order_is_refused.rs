use paddler_typesafe_translation::typesafe_translation_error::TypeSafeTranslationError;
use paddler_typesafe_translation_tests::refusal_of_a_lone_answer_to_two_noul_questions::refusal_of_a_lone_answer_to_two_noul_questions;

#[test]
fn an_answer_out_of_question_order_is_refused() {
    assert!(matches!(
        refusal_of_a_lone_answer_to_two_noul_questions("second")
            .expect("the request with two noul questions must parse"),
        Some(TypeSafeTranslationError::AnswerOutOfOrder {
            expected_question_id,
            received_question_id,
        }) if expected_question_id == "first" && received_question_id == "second"
    ));
}
