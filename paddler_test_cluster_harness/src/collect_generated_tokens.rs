use anyhow::Result;
use futures_util::StreamExt as _;

use paddler_client::inference_message_stream::InferenceMessageStream;
use paddler_messaging::inference_client::message::Message as InferenceMessage;
use paddler_messaging::inference_client::response::Response as InferenceResponse;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_messaging::result_step::ResultStep;
use paddler_messaging::streamable_result::StreamableResult as _;

use crate::cluster_harness_error::ClusterHarnessError;
use crate::collected_generated_tokens::CollectedGeneratedTokens;
use crate::token_result_with_producer::TokenResultWithProducer;

pub async fn collect_generated_tokens(
    mut stream: InferenceMessageStream,
) -> Result<CollectedGeneratedTokens> {
    let mut text = String::new();
    let mut token_results: Vec<TokenResultWithProducer> = Vec::new();

    while let Some(item) = stream.next().await {
        match item.map_err(ClusterHarnessError::TokenStreamFailed)? {
            InferenceMessage::Response(ResponseEnvelope {
                generated_by,
                response: InferenceResponse::GeneratedToken(token_result),
                ..
            }) => {
                if let Some(token_text) = token_result.token_text() {
                    text.push_str(token_text);
                }

                match token_result.into_step() {
                    ResultStep::Final(final_result) => {
                        token_results.push(TokenResultWithProducer {
                            token_result: final_result.into(),
                            generated_by,
                        });

                        break;
                    }
                    ResultStep::Partial(partial_result) => {
                        token_results.push(TokenResultWithProducer {
                            token_result: partial_result.into(),
                            generated_by,
                        });
                    }
                }
            }
            InferenceMessage::Error(error_envelope) => {
                return Err(ClusterHarnessError::TokenStreamReturnedError {
                    error: error_envelope.error,
                }
                .into());
            }
            unexpected_message @ InferenceMessage::Response(_) => {
                return Err(ClusterHarnessError::TokenStreamMessageUnexpected {
                    message: Box::new(unexpected_message),
                }
                .into());
            }
        }
    }

    Ok(CollectedGeneratedTokens {
        text,
        token_results,
    })
}

#[cfg(test)]
mod tests {
    use futures_util::stream::iter;

    use paddler_client::error::Error as ClientError;
    use paddler_messaging::embedding_result::EmbeddingResult;
    use paddler_messaging::inference_client::message::Message as InferenceMessage;
    use paddler_messaging::inference_client::response::Response as InferenceResponse;
    use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;

    use super::collect_generated_tokens;
    use crate::cluster_harness_error::ClusterHarnessError;

    #[tokio::test]
    async fn refuses_a_message_that_is_not_a_generated_token() {
        let embedding_done = || {
            InferenceMessage::Response(ResponseEnvelope {
                generated_by: None,
                request_id: "embedding-request".to_owned(),
                response: InferenceResponse::Embedding(EmbeddingResult::Done),
            })
        };
        let collection_error = collect_generated_tokens(Box::pin(iter([Ok(embedding_done())])))
            .await
            .err()
            .expect("a stream of another kind of message must be refused");

        assert_eq!(
            collection_error
                .downcast_ref::<ClusterHarnessError>()
                .map(ToString::to_string),
            Some(
                ClusterHarnessError::TokenStreamMessageUnexpected {
                    message: Box::new(embedding_done()),
                }
                .to_string()
            )
        );
    }

    #[tokio::test]
    async fn a_token_stream_yielding_an_error_is_an_error() {
        let collection_error =
            collect_generated_tokens(Box::pin(iter([Err(ClientError::ConnectionDropped {
                request_id: "request-1".to_owned(),
            })])))
            .await
            .err()
            .expect("a stream yielding an error must not be collected");

        assert!(matches!(
            collection_error.downcast_ref::<ClusterHarnessError>(),
            Some(ClusterHarnessError::TokenStreamFailed(ClientError::ConnectionDropped {
                request_id
            })) if request_id == "request-1"
        ));
    }
}
