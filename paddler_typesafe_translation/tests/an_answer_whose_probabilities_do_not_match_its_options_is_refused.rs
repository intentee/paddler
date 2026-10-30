use paddler_typesafe_translation::typesafe_translation_error::TypeSafeTranslationError;

use crate::refusal_of_a_lone_answer_to_two_noul_questions::refusal_of_a_lone_answer_to_two_noul_questions;

#[test]
fn an_answer_whose_probabilities_do_not_match_its_options_is_refused() {
    assert_eq!(
        refusal_of_a_lone_answer_to_two_noul_questions("first", vec![1.0]),
        Some(
            TypeSafeTranslationError::AnswerProbabilitiesMismatch {
                expected: 2,
                question_id: "first".to_owned(),
                received: 1,
            }
            .to_string()
        )
    );
}
