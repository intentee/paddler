#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_tests::collect_embedding_results::collect_embedding_results;
use paddler_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::start_in_process_cluster_with_qwen3::start_in_process_cluster_with_qwen3;
use paddler_types::embedding_input_document::EmbeddingInputDocument;
use paddler_types::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_types::request_params::ContinueFromRawPromptParams;
use paddler_types::request_params::GenerateEmbeddingBatchParams;
use reqwest::Client;

#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_rejects_embedding_during_active_generation() -> Result<()> {
    let cluster = start_in_process_cluster_with_qwen3(2).await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let mut generation_stream = inference_client
        .post_continue_from_raw_prompt(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 50,
            raw_prompt: "Tell me a long story about a cat".to_owned(),
        })
        .await?;

    let _first_token = generation_stream
        .next()
        .await
        .ok_or_else(|| anyhow::anyhow!("generation stream must yield at least one message"))?;

    let embedding_outcome = inference_client
        .post_generate_embedding_batch(&GenerateEmbeddingBatchParams {
            input_batch: vec![EmbeddingInputDocument {
                content: "test".to_owned(),
                id: "doc1".to_owned(),
            }],
            normalization_method: EmbeddingNormalizationMethod::None,
        })
        .await;

    if let Ok(embedding_stream) = embedding_outcome {
        let collected = collect_embedding_results(embedding_stream).await;

        if let Ok(collected) = collected {
            assert!(
                !collected.errors.is_empty() || collected.embeddings.is_empty(),
                "embedding request must fail when text-only model is busy generating"
            );
        }
    }

    let _drained = collect_generated_tokens(generation_stream).await;

    cluster.shutdown().await?;

    Ok(())
}
