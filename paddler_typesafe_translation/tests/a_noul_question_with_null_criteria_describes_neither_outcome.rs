use serde_json::json;

use crate::system_one_request_with_questions::system_one_request_with_questions;

#[test]
fn a_noul_question_with_null_criteria_describes_neither_outcome() {
    let translated =
        system_one_request_with_questions(json!({"paid": {"type": "noul", "criteria": null}}))
            .expect("the request must parse")
            .translate()
            .expect("the request must translate");

    assert_eq!(
        translated.decide_params.last_question.options,
        vec!["no".to_owned(), "yes".to_owned()]
    );
}
