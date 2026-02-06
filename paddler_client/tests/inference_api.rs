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
#[tokio::test]
async fn test_generate_embedding_batch() -> paddler_client::Result<()> {
    use paddler_types::embedding_input_document::EmbeddingInputDocument;
    use paddler_types::embedding_normalization_method::EmbeddingNormalizationMethod;
    use paddler_types::request_params::GenerateEmbeddingBatchParams;

    let client = create_paddler_client();
    let params = GenerateEmbeddingBatchParams {
        input_batch: vec![
            EmbeddingInputDocument {
                content: "Hello world".to_string(),
                id: "doc_1".to_string(),
            },
            EmbeddingInputDocument {
                content: "Rust is great".to_string(),
                id: "doc_2".to_string(),
            },
        ],
        normalization_method: EmbeddingNormalizationMethod::None,
    };

    let inference = client.inference();
    let mut stream = inference.generate_embedding_batch(&params).await?;
    let mut embedding_count: usize = 0;

    while let Some(message_result) = stream.next().await {
        let message = message_result?;

        match message {
            InferenceMessage::Response(envelope) => match envelope.response {
                InferenceResponse::Embedding(_) => {
                    embedding_count += 1;
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

    assert_eq!(embedding_count, 2);

    Ok(())
}

#[cfg(feature = "integration_test_inference")]
#[tokio::test(flavor = "multi_thread")]
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
