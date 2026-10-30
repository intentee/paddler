use serde::Deserialize;
use serde::Serialize;

use crate::decision_result::DecisionResult;
use crate::embedding_result::EmbeddingResult;
use crate::generated_token_result::GeneratedTokenResult;
use crate::result_step::ResultStep;
use crate::streamable_result::StreamableResult;

fn response_step<TResult>(result: TResult) -> ResultStep<Response, Response>
where
    TResult: StreamableResult + Into<Response>,
{
    match result.into_step() {
        ResultStep::Final(final_result) => {
            ResultStep::Final(Into::<TResult>::into(final_result).into())
        }
        ResultStep::Partial(partial_result) => {
            ResultStep::Partial(Into::<TResult>::into(partial_result).into())
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum Response {
    Decision(DecisionResult),
    Embedding(EmbeddingResult),
    GeneratedToken(GeneratedTokenResult),
}

impl Response {
    #[must_use]
    pub fn into_step(self) -> ResultStep<Self, Self> {
        match self {
            Self::Decision(decision_result) => response_step(decision_result),
            Self::Embedding(embedding_result) => response_step(embedding_result),
            Self::GeneratedToken(generated_token_result) => response_step(generated_token_result),
        }
    }
}

impl From<DecisionResult> for Response {
    fn from(result: DecisionResult) -> Self {
        Self::Decision(result)
    }
}

impl From<EmbeddingResult> for Response {
    fn from(result: EmbeddingResult) -> Self {
        Self::Embedding(result)
    }
}

impl From<GeneratedTokenResult> for Response {
    fn from(result: GeneratedTokenResult) -> Self {
        Self::GeneratedToken(result)
    }
}
