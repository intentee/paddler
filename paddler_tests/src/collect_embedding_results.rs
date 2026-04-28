use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use futures_util::StreamExt as _;
use paddler_types::embedding::Embedding;
use paddler_types::embedding_result::EmbeddingResult;
use paddler_types::inference_client::Message as InferenceMessage;
use paddler_types::inference_client::Response as InferenceResponse;

use crate::collected_embedding_results::CollectedEmbeddingResults;
use crate::inference_message_stream::InferenceMessageStream;

pub async fn collect_embedding_results(
    mut stream: InferenceMessageStream,
) -> Result<CollectedEmbeddingResults> {
    let mut embeddings: Vec<Embedding> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    let mut saw_done = false;

    while let Some(item) = stream.next().await {
        let message = item.context("embedding stream yielded an error")?;

        match message {
            InferenceMessage::Response(envelope) => match envelope.response {
                InferenceResponse::Embedding(EmbeddingResult::Done) => {
                    saw_done = true;

                    break;
                }
                InferenceResponse::Embedding(EmbeddingResult::Embedding(embedding)) => {
                    embeddings.push(embedding);
                }
                InferenceResponse::Embedding(EmbeddingResult::Error(message)) => {
                    errors.push(message);
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
            },
            InferenceMessage::Error(error_envelope) => {
                return Err(anyhow!(
                    "embedding stream returned JSON-RPC error code {} ({})",
                    error_envelope.error.code,
                    error_envelope.error.description
                ));
            }
        }
    }

    Ok(CollectedEmbeddingResults {
        embeddings,
        errors,
        saw_done,
    })
}
