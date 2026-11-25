use serde::Deserialize;
use serde::Serialize;

use crate::answered_decision_question::AnsweredDecisionQuestion;
use crate::decision_final_result::DecisionFinalResult;
use crate::decision_summary::DecisionSummary;
use crate::oversized_decision_details::OversizedDecisionDetails;
use crate::result_step::ResultStep;
use crate::streamable_result::StreamableResult;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum DecisionResult {
    AnswersUnreadable(String),
    BatchAssemblyFailed(String),
    DecisionsNotServed(String),
    DecodeFailed(String),
    Done(DecisionSummary),
    KvCacheCopyFailed(String),
    KvCacheRemovalFailed(String),
    MediaEvaluationFailed(String),
    ModelNotLoaded(String),
    QuestionAnswered(AnsweredDecisionQuestion),
    RequestExceedsContext(OversizedDecisionDetails),
    RequestRejected(String),
    SchedulerUnavailable(String),
    StopRequested,
}

impl From<AnsweredDecisionQuestion> for DecisionResult {
    fn from(answered_question: AnsweredDecisionQuestion) -> Self {
        Self::QuestionAnswered(answered_question)
    }
}

impl From<DecisionFinalResult> for DecisionResult {
    fn from(final_result: DecisionFinalResult) -> Self {
        match final_result {
            DecisionFinalResult::AnswersUnreadable(detail) => Self::AnswersUnreadable(detail),
            DecisionFinalResult::BatchAssemblyFailed(detail) => Self::BatchAssemblyFailed(detail),
            DecisionFinalResult::DecisionsNotServed(detail) => Self::DecisionsNotServed(detail),
            DecisionFinalResult::DecodeFailed(detail) => Self::DecodeFailed(detail),
            DecisionFinalResult::Done(summary) => Self::Done(summary),
            DecisionFinalResult::KvCacheCopyFailed(detail) => Self::KvCacheCopyFailed(detail),
            DecisionFinalResult::KvCacheRemovalFailed(detail) => Self::KvCacheRemovalFailed(detail),
            DecisionFinalResult::MediaEvaluationFailed(detail) => {
                Self::MediaEvaluationFailed(detail)
            }
            DecisionFinalResult::ModelNotLoaded(detail) => Self::ModelNotLoaded(detail),
            DecisionFinalResult::RequestExceedsContext(details) => {
                Self::RequestExceedsContext(details)
            }
            DecisionFinalResult::RequestRejected(detail) => Self::RequestRejected(detail),
            DecisionFinalResult::SchedulerUnavailable(detail) => Self::SchedulerUnavailable(detail),
            DecisionFinalResult::StopRequested => Self::StopRequested,
        }
    }
}

impl StreamableResult for DecisionResult {
    type Final = DecisionFinalResult;
    type Partial = AnsweredDecisionQuestion;

    fn into_step(self) -> ResultStep<AnsweredDecisionQuestion, DecisionFinalResult> {
        match self {
            Self::AnswersUnreadable(detail) => {
                ResultStep::Final(DecisionFinalResult::AnswersUnreadable(detail))
            }
            Self::BatchAssemblyFailed(detail) => {
                ResultStep::Final(DecisionFinalResult::BatchAssemblyFailed(detail))
            }
            Self::DecisionsNotServed(detail) => {
                ResultStep::Final(DecisionFinalResult::DecisionsNotServed(detail))
            }
            Self::DecodeFailed(detail) => {
                ResultStep::Final(DecisionFinalResult::DecodeFailed(detail))
            }
            Self::Done(summary) => ResultStep::Final(DecisionFinalResult::Done(summary)),
            Self::KvCacheCopyFailed(detail) => {
                ResultStep::Final(DecisionFinalResult::KvCacheCopyFailed(detail))
            }
            Self::KvCacheRemovalFailed(detail) => {
                ResultStep::Final(DecisionFinalResult::KvCacheRemovalFailed(detail))
            }
            Self::MediaEvaluationFailed(detail) => {
                ResultStep::Final(DecisionFinalResult::MediaEvaluationFailed(detail))
            }
            Self::ModelNotLoaded(detail) => {
                ResultStep::Final(DecisionFinalResult::ModelNotLoaded(detail))
            }
            Self::QuestionAnswered(answered_question) => ResultStep::Partial(answered_question),
            Self::RequestExceedsContext(details) => {
                ResultStep::Final(DecisionFinalResult::RequestExceedsContext(details))
            }
            Self::RequestRejected(detail) => {
                ResultStep::Final(DecisionFinalResult::RequestRejected(detail))
            }
            Self::SchedulerUnavailable(detail) => {
                ResultStep::Final(DecisionFinalResult::SchedulerUnavailable(detail))
            }
            Self::StopRequested => ResultStep::Final(DecisionFinalResult::StopRequested),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;

    use llama_cpp_bindings_types::decision_answer::DecisionAnswer;

    use super::DecisionResult;
    use crate::answered_decision_question::AnsweredDecisionQuestion;
    use crate::decision_final_result::DecisionFinalResult;
    use crate::decision_summary::DecisionSummary;
    use crate::oversized_decision_details::OversizedDecisionDetails;
    use crate::result_step::ResultStep;
    use crate::streamable_result::StreamableResult as _;

    fn step_round_trip(decision_result: DecisionResult) -> DecisionResult {
        match decision_result.into_step() {
            ResultStep::Final(final_result) => final_result.into(),
            ResultStep::Partial(answered_question) => answered_question.into(),
        }
    }

    #[test]
    fn answering_a_question_keeps_the_stream_open() {
        let answered_question = AnsweredDecisionQuestion {
            answer: DecisionAnswer::Noul { noul: 0.75 },
            id: "question".to_owned(),
        };

        assert_eq!(
            step_round_trip(DecisionResult::QuestionAnswered(answered_question.clone())),
            DecisionResult::QuestionAnswered(answered_question.clone())
        );
        assert_eq!(
            DecisionResult::QuestionAnswered(answered_question.clone()).into_step(),
            ResultStep::Partial(answered_question)
        );
    }

    #[test]
    fn every_described_failure_ends_the_stream_as_the_same_final_failure() {
        for (decision_result, expected_final_result) in [
            (
                DecisionResult::AnswersUnreadable(String::new()),
                DecisionFinalResult::AnswersUnreadable(String::new()),
            ),
            (
                DecisionResult::BatchAssemblyFailed(String::new()),
                DecisionFinalResult::BatchAssemblyFailed(String::new()),
            ),
            (
                DecisionResult::DecisionsNotServed(String::new()),
                DecisionFinalResult::DecisionsNotServed(String::new()),
            ),
            (
                DecisionResult::DecodeFailed(String::new()),
                DecisionFinalResult::DecodeFailed(String::new()),
            ),
            (
                DecisionResult::KvCacheCopyFailed(String::new()),
                DecisionFinalResult::KvCacheCopyFailed(String::new()),
            ),
            (
                DecisionResult::KvCacheRemovalFailed(String::new()),
                DecisionFinalResult::KvCacheRemovalFailed(String::new()),
            ),
            (
                DecisionResult::MediaEvaluationFailed(String::new()),
                DecisionFinalResult::MediaEvaluationFailed(String::new()),
            ),
            (
                DecisionResult::ModelNotLoaded(String::new()),
                DecisionFinalResult::ModelNotLoaded(String::new()),
            ),
            (
                DecisionResult::RequestRejected(String::new()),
                DecisionFinalResult::RequestRejected(String::new()),
            ),
            (
                DecisionResult::SchedulerUnavailable(String::new()),
                DecisionFinalResult::SchedulerUnavailable(String::new()),
            ),
        ] {
            assert_eq!(
                discriminant(&step_round_trip(decision_result.clone())),
                discriminant(&decision_result)
            );
            assert!(matches!(
                decision_result.into_step(),
                ResultStep::Final(final_result)
                    if discriminant(&final_result) == discriminant(&expected_final_result)
            ));
        }
    }

    #[test]
    fn every_other_result_ends_the_stream_as_its_final_result() {
        for (decision_result, expected_final_result) in [
            (
                DecisionResult::Done(DecisionSummary {
                    input_tokens: 3,
                    processing_milliseconds: 5,
                }),
                DecisionFinalResult::Done(DecisionSummary {
                    input_tokens: 3,
                    processing_milliseconds: 5,
                }),
            ),
            (
                DecisionResult::RequestExceedsContext(OversizedDecisionDetails {
                    context_size: 128,
                    required_tokens: 410,
                }),
                DecisionFinalResult::RequestExceedsContext(OversizedDecisionDetails {
                    context_size: 128,
                    required_tokens: 410,
                }),
            ),
            (
                DecisionResult::StopRequested,
                DecisionFinalResult::StopRequested,
            ),
        ] {
            assert_eq!(step_round_trip(decision_result.clone()), decision_result);
            assert_eq!(
                decision_result.into_step(),
                ResultStep::Final(expected_final_result)
            );
        }
    }
}
