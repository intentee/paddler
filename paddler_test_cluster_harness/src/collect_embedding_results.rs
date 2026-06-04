use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use futures_util::StreamExt as _;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::inference_client::message::Message as InferenceMessage;
use paddler_messaging::inference_client::response::Response as InferenceResponse;

use crate::collected_embedding_results::CollectedEmbeddingResults;
use crate::embedding_with_producer::EmbeddingWithProducer;
use crate::inference_message_stream::InferenceMessageStream;

pub async fn collect_embedding_results(
    mut stream: InferenceMessageStream,
) -> Result<CollectedEmbeddingResults> {
    let mut embeddings: Vec<EmbeddingWithProducer> = Vec::new();
    let mut embeddings_disabled = false;
    let mut errors: Vec<String> = Vec::new();
    let mut embedding_rejected_due_to_active_token_generation_count: usize = 0;
    let mut no_embeddings_produced_count: usize = 0;
    let mut oversized_documents = Vec::new();
    let mut saw_done = false;
    let mut wire_errors = Vec::new();

    while let Some(item) = stream.next().await {
        let message = item.context("embedding stream yielded an error")?;

        match message {
            InferenceMessage::Response(envelope) => {
                let generated_by = envelope.generated_by.clone();

                match envelope.response {
                    InferenceResponse::Embedding(EmbeddingResult::Done) => {
                        saw_done = true;

                        break;
                    }
                    InferenceResponse::Embedding(EmbeddingResult::Embedding(embedding)) => {
                        embeddings.push(EmbeddingWithProducer {
                            embedding,
                            generated_by,
                        });
                    }
                    InferenceResponse::Embedding(EmbeddingResult::DocumentExceedsBatchSize(
                        details,
                    )) => {
                        oversized_documents.push(details);
                    }
                    InferenceResponse::Embedding(EmbeddingResult::EmbeddingsDisabled) => {
                        embeddings_disabled = true;
                    }
                    InferenceResponse::Embedding(EmbeddingResult::Error(message)) => {
                        errors.push(message);
                    }
                    InferenceResponse::Embedding(
                        EmbeddingResult::EmbeddingRejectedDueToActiveTokenGeneration,
                    ) => {
                        embedding_rejected_due_to_active_token_generation_count += 1;
                    }
                    InferenceResponse::Embedding(EmbeddingResult::NoEmbeddingsProduced) => {
                        no_embeddings_produced_count += 1;
                    }
                    InferenceResponse::GeneratedToken(_) => {
                        return Err(anyhow!(
                            "unexpected generated-token response on an embedding stream"
                        ));
                    }
                    InferenceResponse::Timeout => {
                        return Err(anyhow!("embedding request timed out on balancer"));
                    }
                    InferenceResponse::TooManyBufferedRequests => {
                        return Err(anyhow!(
                            "balancer rejected embedding request: too many buffered"
                        ));
                    }
                }
            }
            InferenceMessage::Error(error_envelope) => {
                wire_errors.push(error_envelope.error);
            }
        }
    }

    Ok(CollectedEmbeddingResults {
        embeddings,
        embeddings_disabled,
        errors,
        embedding_rejected_due_to_active_token_generation_count,
        no_embeddings_produced_count,
        oversized_documents,
        saw_done,
        wire_errors,
    })
}

#[cfg(test)]
mod tests {
    use paddler_messaging::embedding::Embedding;
    use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use paddler_messaging::inference_client::message::Message as InferenceMessage;
    use paddler_messaging::inference_client::response::Response as InferenceResponse;
    use paddler_messaging::jsonrpc::error::Error;
    use paddler_messaging::jsonrpc::error_envelope::ErrorEnvelope;
    use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
    use paddler_messaging::oversized_embedding_document_details::OversizedEmbeddingDocumentDetails;
    use paddler_messaging::pooling_type::PoolingType;

    use super::EmbeddingResult;
    use super::collect_embedding_results;
    use crate::inference_message_stream::InferenceMessageStream;

    fn stream(items: Vec<anyhow::Result<InferenceMessage>>) -> InferenceMessageStream {
        Box::pin(futures_util::stream::iter(items))
    }

    fn embedding_message(result: EmbeddingResult) -> InferenceMessage {
        InferenceMessage::Response(ResponseEnvelope {
            generated_by: Some("agent-1".to_owned()),
            request_id: "req".to_owned(),
            response: InferenceResponse::Embedding(result),
        })
    }

    fn sample_embedding() -> Embedding {
        Embedding {
            embedding: vec![0.1, 0.2],
            normalization_method: EmbeddingNormalizationMethod::None,
            pooling_type: PoolingType::Last,
            source_document_id: "doc".to_owned(),
        }
    }

    #[tokio::test]
    async fn collects_embeddings_until_done() {
        let collected = collect_embedding_results(stream(vec![
            Ok(embedding_message(EmbeddingResult::Embedding(
                sample_embedding(),
            ))),
            Ok(embedding_message(EmbeddingResult::Embedding(
                sample_embedding(),
            ))),
            Ok(embedding_message(EmbeddingResult::Done)),
        ]))
        .await
        .unwrap();

        assert_eq!(collected.embeddings.len(), 2);
        assert!(collected.saw_done);
        assert_eq!(
            collected.embeddings[0].generated_by.as_deref(),
            Some("agent-1")
        );
    }

    #[tokio::test]
    async fn records_oversized_documents() {
        let collected = collect_embedding_results(stream(vec![Ok(embedding_message(
            EmbeddingResult::DocumentExceedsBatchSize(OversizedEmbeddingDocumentDetails {
                document_tokens: 5000,
                n_batch: 512,
                source_document_id: "big".to_owned(),
            }),
        ))]))
        .await
        .unwrap();

        assert_eq!(collected.oversized_documents.len(), 1);
        assert_eq!(collected.oversized_documents[0].document_tokens, 5000);
    }

    #[tokio::test]
    async fn records_embeddings_disabled() {
        let collected = collect_embedding_results(stream(vec![Ok(embedding_message(
            EmbeddingResult::EmbeddingsDisabled,
        ))]))
        .await
        .unwrap();

        assert!(collected.embeddings_disabled);
    }

    #[tokio::test]
    async fn records_errors_and_rejections() {
        let collected = collect_embedding_results(stream(vec![
            Ok(embedding_message(EmbeddingResult::Error("boom".to_owned()))),
            Ok(embedding_message(
                EmbeddingResult::EmbeddingRejectedDueToActiveTokenGeneration,
            )),
            Ok(embedding_message(EmbeddingResult::NoEmbeddingsProduced)),
        ]))
        .await
        .unwrap();

        assert_eq!(collected.errors, vec!["boom".to_owned()]);
        assert_eq!(
            collected.embedding_rejected_due_to_active_token_generation_count,
            1
        );
        assert_eq!(collected.no_embeddings_produced_count, 1);
    }

    #[tokio::test]
    async fn rejects_a_generated_token_response() {
        let error = collect_embedding_results(stream(vec![Ok(InferenceMessage::Response(
            ResponseEnvelope {
                generated_by: None,
                request_id: "req".to_owned(),
                response: InferenceResponse::GeneratedToken(GeneratedTokenResult::ContentToken(
                    "x".to_owned(),
                )),
            },
        ))]))
        .await
        .err()
        .unwrap();

        assert!(error.to_string().contains("unexpected generated-token"));
    }

    #[tokio::test]
    async fn rejects_a_timeout() {
        let error = collect_embedding_results(stream(vec![Ok(InferenceMessage::Response(
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
        let error = collect_embedding_results(stream(vec![Ok(InferenceMessage::Response(
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
    async fn records_wire_errors() {
        let collected =
            collect_embedding_results(stream(vec![Ok(InferenceMessage::Error(ErrorEnvelope {
                request_id: "req".to_owned(),
                error: Error {
                    code: -32000,
                    description: "wire failure".to_owned(),
                },
            }))]))
            .await
            .unwrap();

        assert_eq!(collected.wire_errors.len(), 1);
        assert_eq!(collected.wire_errors[0].description, "wire failure");
    }

    #[tokio::test]
    async fn propagates_a_stream_error() {
        let error = collect_embedding_results(stream(vec![Err(anyhow::anyhow!("socket closed"))]))
            .await
            .err()
            .unwrap();

        assert!(
            error
                .to_string()
                .contains("embedding stream yielded an error")
        );
    }
}
