use paddler_typesafe_translation::typesafe_translation_error::TypeSafeTranslationError;

use crate::refusal_of_a_lone_answer_to_two_noul_questions::refusal_of_a_lone_answer_to_two_noul_questions;

#[test]
fn a_question_left_unanswered_is_refused() {
    assert_eq!(
        refusal_of_a_lone_answer_to_two_noul_questions("first", vec![0.5, 0.5]),
        Some(
            TypeSafeTranslationError::AnswerMissing {
                question_id: "second".to_owned(),
            }
            .to_string()
        )
    );
}
