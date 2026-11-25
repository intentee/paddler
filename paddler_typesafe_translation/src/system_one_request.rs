use indexmap::IndexMap;
use llama_cpp_bindings_types::decision_question::DecisionQuestion;
use llama_cpp_bindings_types::decision_request::DecisionRequest;
use serde::Deserialize;
use serde_json::Map;
use serde_json::Value;
use serde_json::from_str;
use serde_json::value::RawValue;

use paddler_messaging::request_params::validates::Validates as _;

use crate::system_one_question::SystemOneQuestion;
use crate::system_one_response_layout::SystemOneResponseLayout;
use crate::translated_system_one_request::TranslatedSystemOneRequest;
use crate::typesafe_translation_error::TypeSafeTranslationError;

const CRITERIA_MAXIMUM: usize = u8::MAX as usize;

fn criteria_in_range(criteria_count: usize) -> bool {
    (1..=CRITERIA_MAXIMUM).contains(&criteria_count)
}

fn criteria_map(criteria: Map<String, Value>) -> IndexMap<String, Value> {
    criteria.into_iter().collect()
}

fn translate_question(
    question_id: &str,
    question: SystemOneQuestion,
) -> Result<DecisionQuestion, TypeSafeTranslationError> {
    match question {
        SystemOneQuestion::Choice {
            criteria,
            instructions,
        } => {
            if !criteria_in_range(criteria.len()) {
                return Err(TypeSafeTranslationError::ChoiceCriteriaOutOfRange {
                    criteria_count: criteria.len(),
                    question_id: question_id.to_owned(),
                });
            }

            Ok(DecisionQuestion::Choice {
                criteria: criteria_map(criteria),
                instructions,
            })
        }
        SystemOneQuestion::Noul {
            criteria,
            instructions,
        } => Ok(DecisionQuestion::Noul {
            criteria: criteria_map(criteria),
            instructions,
        }),
        SystemOneQuestion::Score {
            criteria,
            instructions,
        } => {
            if !criteria_in_range(criteria.len()) {
                return Err(TypeSafeTranslationError::ScoreCriteriaOutOfRange {
                    criteria_count: criteria.len(),
                    question_id: question_id.to_owned(),
                });
            }

            Ok(DecisionQuestion::Score {
                criteria,
                instructions,
            })
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SystemOneRequest {
    pub model: String,
    pub questions: IndexMap<String, Box<RawValue>>,
    pub state: Value,
}

impl SystemOneRequest {
    /// # Errors
    ///
    /// Returns [`TypeSafeTranslationError::QuestionMalformed`] when a question is not a valid choice, noul or score question, [`TypeSafeTranslationError::ChoiceCriteriaOutOfRange`] or [`TypeSafeTranslationError::ScoreCriteriaOutOfRange`] when a choice or score question has no criteria or more than 255 of them, and [`TypeSafeTranslationError::DecisionInvalid`] when the request has no questions.
    pub fn translate(self) -> Result<TranslatedSystemOneRequest, TypeSafeTranslationError> {
        let mut questions = IndexMap::with_capacity(self.questions.len());

        for (question_id, question) in self.questions {
            let question = from_str::<SystemOneQuestion>(question.get()).map_err(|source| {
                TypeSafeTranslationError::QuestionMalformed {
                    question_id: question_id.clone(),
                    source,
                }
            })?;
            let decision_question = translate_question(&question_id, question)?;

            questions.insert(question_id, decision_question);
        }

        let response_layout = SystemOneResponseLayout {
            model: self.model,
            question_ids: questions.keys().cloned().collect(),
        };

        Ok(TranslatedSystemOneRequest {
            decide_params: DecisionRequest {
                images: Vec::new(),
                questions,
                state: self.state,
            }
            .validate()?,
            response_layout,
        })
    }
}
