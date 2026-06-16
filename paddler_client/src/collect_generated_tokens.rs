use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use futures_util::StreamExt as _;
use paddler_messaging::inference_client::message::Message as InferenceMessage;
use paddler_messaging::inference_client::response::Response as InferenceResponse;
use paddler_messaging::streamable_result::StreamableResult as _;

use crate::collected_generated_tokens::CollectedGeneratedTokens;
use crate::inference_message_stream::InferenceMessageStream;
use crate::token_result_with_producer::TokenResultWithProducer;

pub async fn collect_generated_tokens(
    mut stream: InferenceMessageStream,
) -> Result<CollectedGeneratedTokens> {
    let mut text = String::new();
    let mut token_results: Vec<TokenResultWithProducer> = Vec::new();

    while let Some(item) = stream.next().await {
        let message = item.context("inference stream yielded an error")?;

        match message {
            InferenceMessage::Response(envelope) => {
                let generated_by = envelope.generated_by.clone();

                match envelope.response {
                    InferenceResponse::GeneratedToken(token_result) => {
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
                    InferenceResponse::Embedding(_) => {
                        return Err(anyhow!(
                            "unexpected embedding response on a token-generation stream"
                        ));
                    }
                    InferenceResponse::Timeout => {
                        return Err(anyhow!("inference request timed out on balancer"));
                    }
                    InferenceResponse::TooManyBufferedRequests => {
                        return Err(anyhow!("balancer rejected request: too many buffered"));
                    }
                }
            }
            InferenceMessage::Error(error_envelope) => {
                return Err(anyhow!(
                    "inference stream returned JSON-RPC error code {} ({})",
                    error_envelope.error.code,
                    error_envelope.error.description
                ));
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
    use paddler_messaging::embedding_result::EmbeddingResult;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use paddler_messaging::inference_client::message::Message as InferenceMessage;
    use paddler_messaging::inference_client::response::Response as InferenceResponse;
    use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
    use paddler_messaging::jsonrpc::error_envelope::ErrorEnvelope;
    use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;

    use super::collect_generated_tokens;
    use crate::error::Error;
    use crate::error::Result;
    use crate::inference_message_stream::InferenceMessageStream;

    fn stream(items: Vec<Result<InferenceMessage>>) -> InferenceMessageStream {
        Box::pin(futures_util::stream::iter(items))
    }

    fn token(result: GeneratedTokenResult) -> InferenceMessage {
        InferenceMessage::Response(ResponseEnvelope {
            generated_by: None,
            request_id: "req".to_owned(),
            response: InferenceResponse::GeneratedToken(result),
        })
    }

    #[tokio::test]
    async fn accumulates_content_token_text() {
        let collected = collect_generated_tokens(stream(vec![
            Ok(token(GeneratedTokenResult::ContentToken("hel".to_owned()))),
            Ok(token(GeneratedTokenResult::ContentToken("lo".to_owned()))),
        ]))
        .await
        .unwrap();

        assert_eq!(collected.text, "hello");
        assert_eq!(collected.token_results.len(), 2);
    }

    #[tokio::test]
    async fn stops_after_a_terminal_token() {
        let collected = collect_generated_tokens(stream(vec![
            Ok(token(GeneratedTokenResult::ContentToken("hi".to_owned()))),
            Ok(token(GeneratedTokenResult::ImageDecodingFailed(
                "dead".to_owned(),
            ))),
            Ok(token(GeneratedTokenResult::ContentToken(
                "IGNORED".to_owned(),
            ))),
        ]))
        .await
        .unwrap();

        assert_eq!(collected.token_results.len(), 2);
        assert!(collected.text.starts_with("hi"));
        assert!(!collected.text.contains("IGNORED"));
    }

    #[tokio::test]
    async fn rejects_an_embedding_response() {
        let error = collect_generated_tokens(stream(vec![Ok(InferenceMessage::Response(
            ResponseEnvelope {
                generated_by: None,
                request_id: "req".to_owned(),
                response: InferenceResponse::Embedding(EmbeddingResult::Done),
            },
        ))]))
        .await
        .err()
        .unwrap();

        assert!(error.to_string().contains("unexpected embedding response"));
    }

    #[tokio::test]
    async fn rejects_a_timeout() {
        let error = collect_generated_tokens(stream(vec![Ok(InferenceMessage::Response(
            ResponseEnvelope {
                generated_by: None,
                request_id: "req".to_owned(),
                response: InferenceResponse::Timeout,
            },
        ))]))
        .await
        .err()
        .unwrap();

        assert!(error.to_string().contains("timed out"));
    }

    #[tokio::test]
    async fn rejects_too_many_buffered_requests() {
        let error = collect_generated_tokens(stream(vec![Ok(InferenceMessage::Response(
            ResponseEnvelope {
                generated_by: None,
                request_id: "req".to_owned(),
                response: InferenceResponse::TooManyBufferedRequests,
            },
        ))]))
        .await
        .err()
        .unwrap();

        assert!(error.to_string().contains("too many buffered"));
    }

    #[tokio::test]
    async fn propagates_a_wire_error() {
        let error =
            collect_generated_tokens(stream(vec![Ok(InferenceMessage::Error(ErrorEnvelope {
                request_id: "req".to_owned(),
                error: JsonRpcError {
                    code: -32001,
                    description: "rpc failure".to_owned(),
                },
            }))]))
            .await
            .err()
            .unwrap();

        assert!(error.to_string().contains("JSON-RPC error code -32001"));
    }

    #[tokio::test]
    async fn propagates_a_stream_error() {
        let error = collect_generated_tokens(stream(vec![Err(Error::ConnectionSlotEmpty)]))
            .await
            .err()
            .unwrap();

        assert!(
            error
                .to_string()
                .contains("inference stream yielded an error")
        );
    }
}
