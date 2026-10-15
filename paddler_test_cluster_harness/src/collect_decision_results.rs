use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;

use paddler_client::inference_message_stream::InferenceMessageStream;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::inference_client::message::Message as InferenceMessage;
use paddler_messaging::inference_client::response::Response as InferenceResponse;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;

use crate::cluster_harness_error::ClusterHarnessError;
use crate::collected_decision_results::CollectedDecisionResults;

pub async fn collect_decision_results(
    mut stream: InferenceMessageStream,
) -> Result<CollectedDecisionResults> {
    let mut answers = Vec::new();

    while let Some(item) = stream.next().await {
        match item.context("decision stream yielded an error")? {
            InferenceMessage::Response(ResponseEnvelope {
                response: InferenceResponse::Decision(DecisionResult::QuestionAnswered(answer)),
                ..
            }) => answers.push(answer),
            InferenceMessage::Response(ResponseEnvelope {
                response: InferenceResponse::Decision(terminal_result),
                ..
            }) => {
                return Ok(CollectedDecisionResults {
                    answers,
                    terminal_result,
                });
            }
            unexpected_message => {
                return Err(ClusterHarnessError::DecisionStreamMessageUnexpected {
                    message: Box::new(unexpected_message),
                }
                .into());
            }
        }
    }

    Err(ClusterHarnessError::DecisionStreamEndedWithoutTerminalResult.into())
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;

    use futures_util::stream::iter;

    use paddler_client::error::Error as ClientError;
    use paddler_messaging::embedding_result::EmbeddingResult;
    use paddler_messaging::inference_client::message::Message as InferenceMessage;
    use paddler_messaging::inference_client::response::Response as InferenceResponse;
    use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;

    use super::collect_decision_results;
    use crate::cluster_harness_error::ClusterHarnessError;

    #[tokio::test]
    async fn a_decision_stream_ending_without_its_outcome_is_an_error() {
        let collection_error = collect_decision_results(Box::pin(iter(Vec::new())))
            .await
            .expect_err("a stream without an outcome must not be collected");

        assert_eq!(
            collection_error
                .downcast_ref::<ClusterHarnessError>()
                .map(discriminant),
            Some(discriminant(
                &ClusterHarnessError::DecisionStreamEndedWithoutTerminalResult
            ))
        );
    }

    #[tokio::test]
    async fn a_decision_stream_carrying_another_result_is_an_error() {
        let embedding_done = || {
            InferenceMessage::Response(ResponseEnvelope {
                generated_by: None,
                request_id: "request-1".to_owned(),
                response: InferenceResponse::Embedding(EmbeddingResult::Done),
            })
        };
        let collection_error = collect_decision_results(Box::pin(iter([Ok(embedding_done())])))
            .await
            .expect_err("a stream carrying an embedding result must not be collected");

        assert_eq!(
            collection_error
                .downcast_ref::<ClusterHarnessError>()
                .map(ToString::to_string),
            Some(
                ClusterHarnessError::DecisionStreamMessageUnexpected {
                    message: Box::new(embedding_done()),
                }
                .to_string()
            )
        );
    }

    #[tokio::test]
    async fn a_decision_stream_yielding_an_error_is_an_error() {
        let collection_error =
            collect_decision_results(Box::pin(iter([Err(ClientError::ConnectionDropped {
                request_id: "request-1".to_owned(),
            })])))
            .await
            .expect_err("a stream yielding an error must not be collected");

        assert_eq!(
            collection_error.to_string(),
            "decision stream yielded an error"
        );
        assert_eq!(
            collection_error
                .downcast_ref::<ClientError>()
                .map(ToString::to_string),
            Some("Request request-1 failed: connection dropped".to_owned())
        );
    }
}
