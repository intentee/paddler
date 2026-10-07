use std::collections::BTreeSet;

use serde::Deserialize;
use serde::Serialize;

use crate::decision_question::DecisionQuestion;
use crate::request_params::decide_params::DecideParams;
use crate::request_params_validation_error::RequestParamsValidationError;
use crate::validates::Validates;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RawDecideParams {
    pub questions: Vec<DecisionQuestion>,
    pub state: String,
}

impl Validates<DecideParams> for RawDecideParams {
    fn validate(self) -> Result<DecideParams, RequestParamsValidationError> {
        let mut question_ids = BTreeSet::new();

        for question in &self.questions {
            if question.options.is_empty() {
                return Err(
                    RequestParamsValidationError::DecisionQuestionWithoutOptions {
                        question_id: question.id.clone(),
                    },
                );
            }

            if !question_ids.insert(question.id.as_str()) {
                return Err(RequestParamsValidationError::DecisionQuestionIdRepeated {
                    question_id: question.id.clone(),
                });
            }
        }

        let mut leading_questions = self.questions;
        let last_question = leading_questions
            .pop()
            .ok_or(RequestParamsValidationError::DecisionWithoutQuestions)?;

        Ok(DecideParams {
            last_question,
            leading_questions,
            state: self.state,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::RawDecideParams;
    use crate::decision_question::DecisionQuestion;
    use crate::request_params::decide_params::DecideParams;
    use crate::request_params_validation_error::RequestParamsValidationError;
    use crate::validates::Validates as _;

    fn question(id: &str, options: &[&str]) -> DecisionQuestion {
        DecisionQuestion {
            id: id.to_owned(),
            instructions: "Pick one".to_owned(),
            options: options.iter().map(|option| (*option).to_owned()).collect(),
        }
    }

    #[test]
    fn a_decision_needs_at_least_one_question() {
        assert_eq!(
            RawDecideParams {
                questions: Vec::new(),
                state: "state".to_owned(),
            }
            .validate(),
            Err(RequestParamsValidationError::DecisionWithoutQuestions)
        );
    }

    #[test]
    fn every_question_needs_at_least_one_option() {
        assert_eq!(
            RawDecideParams {
                questions: vec![question("first", &["yes"]), question("second", &[])],
                state: "state".to_owned(),
            }
            .validate(),
            Err(
                RequestParamsValidationError::DecisionQuestionWithoutOptions {
                    question_id: "second".to_owned(),
                }
            )
        );
    }

    #[test]
    fn question_ids_must_be_unique() {
        assert_eq!(
            RawDecideParams {
                questions: vec![question("same", &["yes"]), question("same", &["no"])],
                state: "state".to_owned(),
            }
            .validate(),
            Err(RequestParamsValidationError::DecisionQuestionIdRepeated {
                question_id: "same".to_owned(),
            })
        );
    }

    #[test]
    fn valid_questions_keep_their_order_with_the_last_one_apart() {
        assert_eq!(
            RawDecideParams {
                questions: vec![
                    question("first", &["yes", "no"]),
                    question("second", &["a"]),
                    question("third", &["b"]),
                ],
                state: "state".to_owned(),
            }
            .validate(),
            Ok(DecideParams {
                last_question: question("third", &["b"]),
                leading_questions: vec![
                    question("first", &["yes", "no"]),
                    question("second", &["a"]),
                ],
                state: "state".to_owned(),
            })
        );
    }
}
