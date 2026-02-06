#![cfg(any(
    feature = "integration_test_embedding",
    feature = "integration_test_inference"
))]

use futures_util::StreamExt;
use paddler_client::PaddlerClient;
use paddler_types::inference_client::Message as InferenceMessage;
use paddler_types::inference_client::Response as InferenceResponse;
use url::Url;

fn create_paddler_client() -> PaddlerClient {
    let management_url = Url::parse("http://127.0.0.1:8060").expect("valid management URL");
    let inference_url = Url::parse("http://127.0.0.1:8061").expect("valid inference URL");

    PaddlerClient::new(inference_url, management_url, 1)
}

#[cfg(feature = "integration_test_embedding")]
mod embedding_tests {
    use paddler_types::embedding::Embedding;
    use paddler_types::embedding_input_document::EmbeddingInputDocument;
    use paddler_types::embedding_normalization_method::EmbeddingNormalizationMethod;
    use paddler_types::embedding_result::EmbeddingResult;
    use paddler_types::request_params::GenerateEmbeddingBatchParams;

    use super::*;

    async fn collect_embeddings(
        params: &GenerateEmbeddingBatchParams,
    ) -> paddler_client::Result<Vec<Embedding>> {
        let client = create_paddler_client();
        let inference = client.inference();
        let mut stream = inference.generate_embedding_batch(params).await?;
        let mut embeddings: Vec<Embedding> = Vec::new();

        while let Some(message_result) = stream.next().await {
            let message = message_result?;

            match message {
                InferenceMessage::Response(envelope) => match envelope.response {
                    InferenceResponse::Embedding(EmbeddingResult::Embedding(embedding)) => {
                        embeddings.push(embedding);
                    }
                    InferenceResponse::Embedding(EmbeddingResult::Done) => {}
                    InferenceResponse::Embedding(EmbeddingResult::Error(error_description)) => {
                        panic!("embedding error: {error_description}");
                    }
                    other => {
                        panic!("unexpected response variant: {other:?}");
                    }
                },
                InferenceMessage::Error(error_envelope) => {
                    panic!(
                        "server returned error: {} (code {})",
                        error_envelope.error.description, error_envelope.error.code
                    );
                }
            }
        }

        Ok(embeddings)
    }

    fn make_document(id: &str, content: &str) -> EmbeddingInputDocument {
        EmbeddingInputDocument {
            content: content.to_string(),
            id: id.to_string(),
        }
    }

    fn make_params(documents: Vec<EmbeddingInputDocument>) -> GenerateEmbeddingBatchParams {
        GenerateEmbeddingBatchParams {
            input_batch: documents,
            normalization_method: EmbeddingNormalizationMethod::None,
        }
    }

    #[tokio::test]
    async fn test_single_document_returns_one_embedding() -> paddler_client::Result<()> {
        let params = make_params(vec![make_document("only_doc", "The quick brown fox.")]);
        let embeddings = collect_embeddings(&params).await?;

        assert_eq!(embeddings.len(), 1);
        assert_eq!(embeddings[0].source_document_id, "only_doc");
        assert!(
            !embeddings[0].embedding.is_empty(),
            "embedding vector must not be empty"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_two_documents_return_two_embeddings_with_correct_ids()
    -> paddler_client::Result<()> {
        let params = make_params(vec![
            make_document("1", "The quick brown fox jumped over a lazy dog."),
            make_document("2", "The quick brown dog jumped over a lazy fox."),
        ]);
        let embeddings = collect_embeddings(&params).await?;

        assert_eq!(embeddings.len(), 2);

        let source_ids: Vec<&str> = embeddings
            .iter()
            .map(|embedding| embedding.source_document_id.as_str())
            .collect();

        assert!(source_ids.contains(&"1"));
        assert!(source_ids.contains(&"2"));

        Ok(())
    }

    #[tokio::test]
    async fn test_all_embeddings_have_same_dimensions() -> paddler_client::Result<()> {
        let params = make_params(vec![
            make_document("short", "Hello"),
            make_document("medium", "The quick brown fox jumped over a lazy dog."),
            make_document(
                "long",
                "Rust is a systems programming language focused on safety, speed, and concurrency. It achieves memory safety without garbage collection.",
            ),
        ]);
        let embeddings = collect_embeddings(&params).await?;

        assert_eq!(embeddings.len(), 3);

        let first_dimension = embeddings[0].embedding.len();

        assert!(first_dimension > 0, "embedding dimension must be positive");

        for embedding in &embeddings {
            assert_eq!(
                embedding.embedding.len(),
                first_dimension,
                "all embeddings must have the same dimension, but {} has {} instead of {}",
                embedding.source_document_id,
                embedding.embedding.len(),
                first_dimension
            );
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_identical_documents_produce_identical_embeddings() -> paddler_client::Result<()> {
        let repeated_content = "Deterministic embedding output test.";
        let params = make_params(vec![
            make_document("first", repeated_content),
            make_document("second", repeated_content),
        ]);
        let embeddings = collect_embeddings(&params).await?;

        assert_eq!(embeddings.len(), 2);

        let first = embeddings
            .iter()
            .find(|embedding| embedding.source_document_id == "first")
            .expect("first embedding missing");
        let second = embeddings
            .iter()
            .find(|embedding| embedding.source_document_id == "second")
            .expect("second embedding missing");

        assert_eq!(
            first.embedding, second.embedding,
            "identical documents must produce identical embedding vectors"
        );

        Ok(())
    }
}

#[cfg(feature = "integration_test_inference")]
#[tokio::test]
async fn test_continue_from_raw_prompt() -> paddler_client::Result<()> {
    use paddler_types::request_params::ContinueFromRawPromptParams;
    use paddler_types::streamable_result::StreamableResult;

    let client = create_paddler_client();
    let params = ContinueFromRawPromptParams {
        max_tokens: 16,
        raw_prompt: "The capital of France is".to_string(),
    };

    let inference = client.inference();
    let mut stream = inference.continue_from_raw_prompt(params).await?;
    let mut token_count: usize = 0;

    while let Some(message_result) = stream.next().await {
        let message = message_result?;

        match message {
            InferenceMessage::Response(envelope) => match envelope.response {
                InferenceResponse::GeneratedToken(token_result) => {
                    token_count += 1;

                    if token_result.is_done() {
                        break;
                    }
                }
                other => {
                    panic!("unexpected response variant: {other:?}");
                }
            },
            InferenceMessage::Error(error_envelope) => {
                panic!(
                    "server returned error: {} (code {})",
                    error_envelope.error.description, error_envelope.error.code
                );
            }
        }
    }

    assert!(
        token_count > 0,
        "expected at least one token from the stream"
    );

    Ok(())
}
