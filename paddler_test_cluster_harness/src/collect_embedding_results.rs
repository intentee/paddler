use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use futures_util::StreamExt as _;
use paddler_client::inference_message_stream::InferenceMessageStream;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::inference_client::message::Message as InferenceMessage;
use paddler_messaging::inference_client::response::Response as InferenceResponse;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;

use crate::collected_embedding_results::CollectedEmbeddingResults;
use crate::embedding_with_producer::EmbeddingWithProducer;

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
        match item.context("embedding stream yielded an error")? {
            InferenceMessage::Response(ResponseEnvelope {
                generated_by,
                response: InferenceResponse::Embedding(embedding_result),
                ..
            }) => match embedding_result {
                EmbeddingResult::Done => {
                    saw_done = true;

                    break;
                }
                EmbeddingResult::Embedding(embedding) => {
                    embeddings.push(EmbeddingWithProducer {
                        embedding,
                        generated_by,
                    });
                }
                EmbeddingResult::DocumentExceedsBatchSize(details) => {
                    oversized_documents.push(details);
                }
                EmbeddingResult::EmbeddingsDisabled => {
                    embeddings_disabled = true;
                }
                EmbeddingResult::Error(message) => {
                    errors.push(message);
                }
                EmbeddingResult::EmbeddingRejectedDueToActiveTokenGeneration => {
                    embedding_rejected_due_to_active_token_generation_count += 1;
                }
                EmbeddingResult::NoEmbeddingsProduced => {
                    no_embeddings_produced_count += 1;
                }
            },
            InferenceMessage::Error(error_envelope) => {
                wire_errors.push(error_envelope.error);
            }
            unexpected_message => {
                return Err(anyhow!(
                    "unexpected message on an embedding stream: {unexpected_message:?}"
                ));
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
