#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::request_params::ContinueFromRawPromptParams;
use paddler_messaging::request_params::GenerateEmbeddingBatchParams;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn continuous_batch_rejects_embedding_during_active_generation() -> Result<()> {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(2)]).await?;

    let mut generation_stream = cluster
        .continue_from_raw_prompt_stream(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 50,
            raw_prompt: "Tell me a long story about a cat".to_owned(),
        })
        .await?;

    let _first_token = generation_stream
        .next()
        .await
        .ok_or_else(|| anyhow::anyhow!("generation stream must yield at least one message"))?;

    let embedding_outcome = cluster
        .generate_embedding_batch(&GenerateEmbeddingBatchParams {
            input_batch: vec![EmbeddingInputDocument {
                content: "test".to_owned(),
                id: "doc1".to_owned(),
            }],
            normalization_method: EmbeddingNormalizationMethod::None,
        })
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
