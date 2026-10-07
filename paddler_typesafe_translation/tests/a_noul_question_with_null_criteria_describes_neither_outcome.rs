use serde_json::from_value;
use serde_json::json;

use paddler_typesafe_translation::system_one_request::SystemOneRequest;

#[test]
fn a_noul_question_with_null_criteria_describes_neither_outcome() {
    let translated = from_value::<SystemOneRequest>(json!({
        "state": "state",
        "questions": {"paid": {"type": "noul", "criteria": null}},
    }))
    .expect("the request must parse")
    .translate()
    .expect("the request must translate");

    assert_eq!(
        translated.decide_params.last_question.options,
        vec!["no".to_owned(), "yes".to_owned()]
    );
}
