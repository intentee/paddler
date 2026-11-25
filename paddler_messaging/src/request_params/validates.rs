use llama_cpp_bindings_types::decision_question::DecisionQuestion;
use llama_cpp_bindings_types::decision_request::DecisionRequest;
use serde_json::Value;

use crate::request_params::decide_params::DecideParams;
use crate::request_params_validation_error::RequestParamsValidationError;

const NUL_BYTE: char = '\0';

fn entries_contain_a_nul_byte<'entries>(
    mut entries: impl Iterator<Item = (&'entries String, &'entries Value)>,
) -> bool {
    entries.any(|(key, value)| key.contains(NUL_BYTE) || contains_a_nul_byte(value))
}

fn contains_a_nul_byte(value: &Value) -> bool {
    match value {
        Value::Array(items) => items.iter().any(contains_a_nul_byte),
        Value::Object(fields) => entries_contain_a_nul_byte(fields.iter()),
        Value::String(text) => text.contains(NUL_BYTE),
        Value::Bool(_) | Value::Null | Value::Number(_) => false,
    }
}

fn question_contains_a_nul_byte(question: &DecisionQuestion) -> bool {
    match question {
        DecisionQuestion::Choice {
            criteria,
            instructions,
        }
        | DecisionQuestion::Noul {
            criteria,
            instructions,
        } => contains_a_nul_byte(instructions) || entries_contain_a_nul_byte(criteria.iter()),
        DecisionQuestion::Score {
            criteria,
            instructions,
        } => contains_a_nul_byte(instructions) || criteria.iter().any(contains_a_nul_byte),
    }
}

pub trait Validates<TOutput> {
    /// # Errors
    ///
    /// Returns [`RequestParamsValidationError::DecisionWithoutQuestions`] when a decision asks no question, [`RequestParamsValidationError::DecisionStateContainsNulByte`] or [`RequestParamsValidationError::DecisionQuestionContainsNulByte`] when the text of a decision contains a NUL byte, [`RequestParamsValidationError::ToolCallParsingWithoutTools`] when tool call parsing is requested without tools, and [`RequestParamsValidationError::RequiredFieldNotInProperties`] when a tool parameters schema requires a field it does not define.
    fn validate(self) -> Result<TOutput, RequestParamsValidationError>;
}

impl Validates<DecideParams> for DecisionRequest {
    fn validate(self) -> Result<DecideParams, RequestParamsValidationError> {
        if self.questions.is_empty() {
            return Err(RequestParamsValidationError::DecisionWithoutQuestions);
        }

        if contains_a_nul_byte(&self.state) {
            return Err(RequestParamsValidationError::DecisionStateContainsNulByte);
        }

        if let Some((question_id, _question)) = self
            .questions
            .iter()
            .find(|(_question_id, question)| question_contains_a_nul_byte(question))
        {
            return Err(
                RequestParamsValidationError::DecisionQuestionContainsNulByte {
                    question_id: question_id.clone(),
                },
            );
        }

        Ok(DecideParams { request: self })
    }
}

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;
    use llama_cpp_bindings_types::decision_question::DecisionQuestion;
    use llama_cpp_bindings_types::decision_request::DecisionRequest;
    use serde_json::Value;
    use serde_json::json;

    use super::Validates as _;
    use crate::request_params_validation_error::RequestParamsValidationError;

    #[test]
    fn a_decision_whose_state_contains_a_nul_byte_is_refused() {
        assert_eq!(
            DecisionRequest {
                images: Vec::new(),
                questions: IndexMap::from([(
                    "paid".to_owned(),
                    DecisionQuestion::Noul {
                        criteria: IndexMap::new(),
                        instructions: json!("Was the invoice paid?"),
                    },
                )]),
                state: json!({"orders": [1, true, null, {"note": "late \0 payment"}]}),
            }
            .validate(),
            Err(RequestParamsValidationError::DecisionStateContainsNulByte)
        );
    }

    #[test]
    fn a_decision_question_containing_a_nul_byte_is_refused_with_its_id() {
        assert_eq!(
            DecisionRequest {
                images: Vec::new(),
                questions: IndexMap::from([
                    (
                        "sentiment".to_owned(),
                        DecisionQuestion::Choice {
                            criteria: IndexMap::from([("positive".to_owned(), Value::Null)]),
                            instructions: json!("How does the customer feel?"),
                        },
                    ),
                    (
                        "paid".to_owned(),
                        DecisionQuestion::Noul {
                            criteria: IndexMap::new(),
                            instructions: json!("Was the invoice paid?"),
                        },
                    ),
                    (
                        "urgency".to_owned(),
                        DecisionQuestion::Score {
                            criteria: vec![json!("Can wait"), json!("Needs \0 attention")],
                            instructions: json!("How urgent is it?"),
                        },
                    ),
                ]),
                state: json!("Ada paid late."),
            }
            .validate(),
            Err(
                RequestParamsValidationError::DecisionQuestionContainsNulByte {
                    question_id: "urgency".to_owned()
                }
            )
        );
    }

    #[test]
    fn a_decision_needs_at_least_one_question() {
        assert_eq!(
            DecisionRequest {
                images: Vec::new(),
                questions: IndexMap::new(),
                state: json!("state"),
            }
            .validate(),
            Err(RequestParamsValidationError::DecisionWithoutQuestions)
        );
    }
}
