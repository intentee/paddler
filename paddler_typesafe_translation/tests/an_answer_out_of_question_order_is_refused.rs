use paddler_typesafe_translation::typesafe_translation_error::TypeSafeTranslationError;

use crate::refusal_of_a_lone_answer_to_two_noul_questions::refusal_of_a_lone_answer_to_two_noul_questions;

#[test]
fn an_answer_out_of_question_order_is_refused() {
    assert_eq!(
        refusal_of_a_lone_answer_to_two_noul_questions("second", vec![0.5, 0.5]),
        Some(
            TypeSafeTranslationError::AnswerOutOfOrder {
                expected_question_id: "first".to_owned(),
                received_question_id: "second".to_owned(),
            }
            .to_string()
        )
    );
}
