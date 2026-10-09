use std::ops::RangeInclusive;

use serde::Deserialize;
use serde_json::Map;
use serde_json::Value;
use serde_json::from_value;

use paddler_messaging::decision_question::DecisionQuestion;
use paddler_messaging::request_params::decide_params::DecideParams;

use crate::render_json_content::render_json_content;
use crate::system_one_answer_layout::SystemOneAnswerLayout;
use crate::system_one_question::SystemOneQuestion;
use crate::translated_system_one_question::TranslatedSystemOneQuestion;
use crate::translated_system_one_request::TranslatedSystemOneRequest;
use crate::typesafe_translation_error::TypeSafeTranslationError;

const CRITERIA_RANGE: RangeInclusive<usize> = 1..=255;

fn default_model() -> String {
    "kev-latest".to_owned()
}

fn option_text(name: &str, description: Option<&Value>) -> String {
    match description {
        None | Some(Value::Null) => name.to_owned(),
        Some(Value::String(text)) if text.is_empty() => name.to_owned(),
        Some(description) => format!("{name}: {}", render_json_content(description, 0)),
    }
}

fn translate_question(
    question_id: String,
    question: SystemOneQuestion,
) -> Result<TranslatedSystemOneQuestion, TypeSafeTranslationError> {
    match question {
        SystemOneQuestion::Choice {
            criteria,
            instructions,
        } => {
            if !CRITERIA_RANGE.contains(&criteria.len()) {
                return Err(TypeSafeTranslationError::ChoiceCriteriaOutOfRange {
                    criteria_count: criteria.len(),
                    question_id,
                });
            }

            Ok(TranslatedSystemOneQuestion {
                decision_question: DecisionQuestion {
                    id: question_id.clone(),
                    instructions: render_json_content(&instructions, 0),
                    options: criteria
                        .iter()
                        .map(|(name, description)| option_text(name, Some(description)))
                        .collect(),
                },
                answer_layout: SystemOneAnswerLayout::Choice {
                    id: question_id,
                    keys: criteria.keys().cloned().collect(),
                },
            })
        }
        SystemOneQuestion::Noul {
            criteria,
            instructions,
        } => Ok(TranslatedSystemOneQuestion {
            decision_question: DecisionQuestion {
                id: question_id.clone(),
                instructions: render_json_content(&instructions, 0),
                options: vec![
                    option_text("no", criteria.get("false")),
                    option_text("yes", criteria.get("true")),
                ],
            },
            answer_layout: SystemOneAnswerLayout::Noul { id: question_id },
        }),
        SystemOneQuestion::Score {
            criteria,
            instructions,
        } => {
            if !CRITERIA_RANGE.contains(&criteria.len()) {
                return Err(TypeSafeTranslationError::ScoreCriteriaOutOfRange {
                    criteria_count: criteria.len(),
                    question_id,
                });
            }

            let legend: Vec<String> = criteria
                .iter()
                .map(|level| render_json_content(level, 0))
                .collect();

            Ok(TranslatedSystemOneQuestion {
                decision_question: DecisionQuestion {
                    id: question_id.clone(),
                    instructions: render_json_content(&instructions, 0),
                    options: legend.clone(),
                },
                answer_layout: SystemOneAnswerLayout::Score {
                    id: question_id,
                    legend,
                },
            })
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SystemOneRequest {
    #[serde(default = "default_model")]
    pub model: String,
    pub questions: Map<String, Value>,
    pub state: Value,
}

impl SystemOneRequest {
    pub fn translate(self) -> Result<TranslatedSystemOneRequest, TypeSafeTranslationError> {
        let mut answer_layouts = Vec::with_capacity(self.questions.len());
        let mut questions = Vec::with_capacity(self.questions.len());

        for (question_id, question) in self.questions {
            let question = from_value::<SystemOneQuestion>(question).map_err(|source| {
                TypeSafeTranslationError::QuestionMalformed {
                    question_id: question_id.clone(),
                    source,
                }
            })?;
            let TranslatedSystemOneQuestion {
                answer_layout,
                decision_question,
            } = translate_question(question_id, question)?;

            questions.push(decision_question);
            answer_layouts.push(answer_layout);
        }

        let last_question = questions
            .pop()
            .ok_or(TypeSafeTranslationError::QuestionsMissing)?;

        Ok(TranslatedSystemOneRequest {
            answer_layouts,
            decide_params: DecideParams {
                last_question,
                leading_questions: questions,
                state: render_json_content(&self.state, 0),
            },
            model: self.model,
        })
    }
}
