use serde::Deserialize;
use serde::Serialize;

use crate::decision_result::DecisionResult;
use crate::embedding_result::EmbeddingResult;
use crate::generated_token_result::GeneratedTokenResult;

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum Response {
    Decision(DecisionResult),
    Embedding(EmbeddingResult),
    GeneratedToken(GeneratedTokenResult),
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

impl TryFrom<Response> for DecisionResult {
    type Error = Response;

    fn try_from(response: Response) -> Result<Self, Response> {
        match response {
            Response::Decision(result) => Ok(result),
            other_response @ (Response::Embedding(_) | Response::GeneratedToken(_)) => {
                Err(other_response)
            }
        }
    }
}

impl TryFrom<Response> for GeneratedTokenResult {
    type Error = Response;

    fn try_from(response: Response) -> Result<Self, Response> {
        match response {
            Response::GeneratedToken(result) => Ok(result),
            other_response @ (Response::Decision(_) | Response::Embedding(_)) => {
                Err(other_response)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Response;
    use crate::decision_result::DecisionResult;
    use crate::embedding_result::EmbeddingResult;
    use crate::generated_token_result::GeneratedTokenResult;

    #[test]
    fn a_decision_response_yields_its_decision_result() {
        assert_eq!(
            DecisionResult::try_from(Response::Decision(DecisionResult::ModelNotLoaded(
                "not loaded".to_owned()
            ))),
            Ok(DecisionResult::ModelNotLoaded("not loaded".to_owned()))
        );
    }

    #[test]
    fn a_response_of_another_mode_is_returned_instead_of_a_decision_result() {
        assert_eq!(
            DecisionResult::try_from(Response::Embedding(EmbeddingResult::Done)),
            Err(Response::Embedding(EmbeddingResult::Done))
        );
    }

    #[test]
    fn a_generated_token_response_yields_its_generated_token_result() {
        assert_eq!(
            GeneratedTokenResult::try_from(Response::GeneratedToken(
                GeneratedTokenResult::ModelNotLoaded("not loaded".to_owned())
            )),
            Ok(GeneratedTokenResult::ModelNotLoaded(
                "not loaded".to_owned()
            ))
        );
    }

    #[test]
    fn a_response_of_another_mode_is_returned_instead_of_a_generated_token_result() {
        assert_eq!(
            GeneratedTokenResult::try_from(Response::Embedding(EmbeddingResult::Done)),
            Err(Response::Embedding(EmbeddingResult::Done))
        );
    }
}
