use serde_json::Map;

use paddler_messaging::answered_decision_question::AnsweredDecisionQuestion;
use paddler_messaging::decision_summary::DecisionSummary;

use crate::system_one_answer::SystemOneAnswer;
use crate::system_one_response::SystemOneResponse;
use crate::system_one_usage::SystemOneUsage;
use crate::typesafe_translation_error::TypeSafeTranslationError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SystemOneResponseLayout {
    pub model: String,
    pub question_ids: Vec<String>,
}

impl SystemOneResponseLayout {
    /// # Errors
    ///
    /// Returns [`TypeSafeTranslationError::AnswerMissing`] when the agent answered fewer questions than were asked, and [`TypeSafeTranslationError::AnswerOutOfOrder`] when an answer belongs to a question other than the one due.
    pub fn respond(
        &self,
        answered_questions: Vec<AnsweredDecisionQuestion>,
        DecisionSummary {
            input_tokens,
            processing_milliseconds,
        }: &DecisionSummary,
    ) -> Result<SystemOneResponse, TypeSafeTranslationError> {
        let mut answers = Map::new();
        let mut answered_questions = answered_questions.into_iter();

        for question_id in &self.question_ids {
            let Some(AnsweredDecisionQuestion { answer, id }) = answered_questions.next() else {
                return Err(TypeSafeTranslationError::AnswerMissing {
                    question_id: question_id.clone(),
                });
            };

            if id != *question_id {
                return Err(TypeSafeTranslationError::AnswerOutOfOrder {
                    expected_question_id: question_id.clone(),
                    received_question_id: id,
                });
            }

            answers.insert(
                id,
                SystemOneAnswer {
                    decision_answer: answer,
                }
                .into_value(),
            );
        }

        Ok(SystemOneResponse {
            model: self.model.clone(),
            answers,
            usage: SystemOneUsage {
                input_tokens: *input_tokens,
                output_tokens: 0,
            },
            latency_ms: *processing_milliseconds,
        })
    }
}
