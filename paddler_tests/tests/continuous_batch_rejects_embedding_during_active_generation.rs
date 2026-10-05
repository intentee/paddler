#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use anyhow::anyhow;
use futures_util::StreamExt as _;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::collect_generated_tokens::collect_generated_tokens;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_rejects_embedding_during_active_generation() -> Result<()> {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(2)]).await?;

    let mut generation_stream = cluster
        .continue_from_raw_prompt_stream(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: 50,
                raw_prompt: "Tell me a long story about a cat".to_owned(),
            },
        )
        .await?;

    let _first_token = generation_stream
        .next()
        .await
        .ok_or_else(|| anyhow!("generation stream must yield at least one message"))?;

    let embedding_outcome = cluster
        .generate_embedding_batch(
            CancellationToken::new(),
            &GenerateEmbeddingBatchParams {
                input_batch: vec![EmbeddingInputDocument {
                    content: "test".to_owned(),
                    id: "doc1".to_owned(),
                }],
                normalization_method: EmbeddingNormalizationMethod::None,
            },
        )
        .await;

    if let Ok(collected) = embedding_outcome {
        assert!(
            !collected.errors.is_empty() || collected.embeddings.is_empty(),
            "embedding request must fail when text-only model is busy generating"
        );
    }

    let _drained = collect_generated_tokens(generation_stream).await;

    cluster.shutdown().await?;

    Ok(())
}
