use std::collections::BTreeSet;

use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;

use paddler_client::inference_message_stream::InferenceMessageStream;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::inference_client::message::Message as InferenceMessage;
use paddler_messaging::inference_client::response::Response as InferenceResponse;
use paddler_messaging::jsonrpc::error_envelope::ErrorEnvelope;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;

use crate::cluster_harness_error::ClusterHarnessError;
use crate::collected_embedding_results::CollectedEmbeddingResults;
use crate::embedding_with_producer::EmbeddingWithProducer;

pub async fn collect_embedding_results(
    mut stream: InferenceMessageStream,
) -> Result<CollectedEmbeddingResults> {
    let mut embeddings: Vec<EmbeddingWithProducer> = Vec::new();
    let mut embeddings_disabled = false;
    let mut errors: Vec<String> = Vec::new();
    let mut embedding_rejected_due_to_active_token_generation_count: usize = 0;
    let mut model_not_loaded_count: usize = 0;
    let mut no_embeddings_produced_count: usize = 0;
    let mut oversized_documents = Vec::new();
    let mut request_ids = BTreeSet::new();
    let mut saw_done = false;
    let mut wire_errors = Vec::new();

    while let Some(item) = stream.next().await {
        match item.context("embedding stream yielded an error")? {
            InferenceMessage::Response(ResponseEnvelope {
                generated_by,
                request_id,
                response: InferenceResponse::Embedding(embedding_result),
            }) => {
                request_ids.insert(request_id);

                match embedding_result {
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
                    EmbeddingResult::ModelNotLoaded(_) => {
                        model_not_loaded_count += 1;
                    }
                    EmbeddingResult::NoEmbeddingsProduced => {
                        no_embeddings_produced_count += 1;
                    }
                }
            }
            InferenceMessage::Error(ErrorEnvelope { error, request_id }) => {
                request_ids.insert(request_id);
                wire_errors.push(error);
            }
            unexpected_message => {
                return Err(ClusterHarnessError::EmbeddingStreamMessageUnexpected {
                    message: Box::new(unexpected_message),
                }
                .into());
            }
        }
    }

    Ok(CollectedEmbeddingResults {
        embeddings,
        embeddings_disabled,
        errors,
        embedding_rejected_due_to_active_token_generation_count,
        model_not_loaded_count,
        no_embeddings_produced_count,
        oversized_documents,
        request_ids,
        saw_done,
        wire_errors,
    })
}
