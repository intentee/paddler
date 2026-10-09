use serde_json::Map;

use paddler_messaging::decision_answer::DecisionAnswer;
use paddler_messaging::decision_summary::DecisionSummary;
use paddler_messaging::request_params::decide_params::DecideParams;

use crate::system_one_answer_layout::SystemOneAnswerLayout;
use crate::system_one_response::SystemOneResponse;
use crate::system_one_usage::SystemOneUsage;
use crate::typesafe_translation_error::TypeSafeTranslationError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TranslatedSystemOneRequest {
    pub answer_layouts: Vec<SystemOneAnswerLayout>,
    pub decide_params: DecideParams,
    pub model: String,
}

impl TranslatedSystemOneRequest {
    pub fn respond(
        &self,
        decision_answers: &[DecisionAnswer],
        DecisionSummary {
            input_tokens,
            processing_milliseconds,
        }: &DecisionSummary,
    ) -> Result<SystemOneResponse, TypeSafeTranslationError> {
        let mut answers = Map::new();
        let mut decision_answers = decision_answers.iter();

        for answer_layout in &self.answer_layouts {
            let Some(DecisionAnswer { id, probabilities }) = decision_answers.next() else {
                return Err(TypeSafeTranslationError::AnswerMissing {
                    question_id: answer_layout.id().to_owned(),
                });
            };

            if id != answer_layout.id() {
                return Err(TypeSafeTranslationError::AnswerOutOfOrder {
                    expected_question_id: answer_layout.id().to_owned(),
                    received_question_id: id.clone(),
                });
            }

            answers.insert(
                id.clone(),
                answer_layout.answer(probabilities)?.into_value(),
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
