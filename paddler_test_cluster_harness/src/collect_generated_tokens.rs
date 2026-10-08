use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;

use paddler_client::inference_message_stream::InferenceMessageStream;
use paddler_messaging::inference_client::message::Message as InferenceMessage;
use paddler_messaging::inference_client::response::Response as InferenceResponse;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
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
        match item.context("inference stream yielded an error")? {
            InferenceMessage::Response(ResponseEnvelope {
                generated_by,
                response: InferenceResponse::GeneratedToken(token_result),
                ..
            }) => {
                if let Some(token_text) = token_result.token_text() {
                    text.push_str(token_text);
                }

                let is_done = token_result.is_done();

                token_results.push(TokenResultWithProducer {
                    token_result,
                    generated_by,
                });

                if is_done {
                    break;
                }
            }
            InferenceMessage::Error(error_envelope) => {
                return Err(ClusterHarnessError::TokenStreamReturnedError {
                    error: error_envelope.error,
                }
                .into());
            }
            unexpected_message => {
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

    use paddler_messaging::embedding_result::EmbeddingResult;
    use paddler_messaging::inference_client::message::Message as InferenceMessage;
    use paddler_messaging::inference_client::response::Response as InferenceResponse;
    use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;

    use super::collect_generated_tokens;
    use crate::cluster_harness_error::ClusterHarnessError;

    #[tokio::test]
    async fn refuses_a_message_that_is_not_a_generated_token() {
        let collection_error = collect_generated_tokens(Box::pin(iter([Ok(
            InferenceMessage::Response(ResponseEnvelope {
                generated_by: None,
                request_id: "embedding-request".to_owned(),
                response: InferenceResponse::Embedding(EmbeddingResult::Done),
            }),
        )])))
        .await
        .err()
        .expect("a stream of another kind of message must be refused");

        assert!(matches!(
            collection_error.downcast_ref::<ClusterHarnessError>(),
            Some(ClusterHarnessError::TokenStreamMessageUnexpected { message })
                if matches!(
                    message.as_ref(),
                    InferenceMessage::Response(ResponseEnvelope { request_id, .. })
                        if request_id == "embedding-request"
                )
        ));
    }
}
