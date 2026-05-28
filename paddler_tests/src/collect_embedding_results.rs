use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use futures_util::StreamExt as _;
use paddler::embedding_result::EmbeddingResult;
use paddler::balancer::inference_client::Message as InferenceMessage;
use paddler::balancer::inference_client::Response as InferenceResponse;

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
