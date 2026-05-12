#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use std::time::Duration;

use anyhow::Result;
use anyhow::anyhow;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::collect_embedding_results::collect_embedding_results;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::qwen3_embedding_cluster_params::Qwen3EmbeddingClusterParams;
use paddler_tests::start_subprocess_cluster_with_qwen3_embedding::start_subprocess_cluster_with_qwen3_embedding;
use paddler_types::embedding_input_document::EmbeddingInputDocument;
use paddler_types::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::GenerateEmbeddingBatchParams;
use reqwest::Client;
use tokio::time::timeout;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn balancer_emits_overflow_errors_when_embedding_burst_exceeds_max_buffered_requests()
-> Result<()> {
    const TOTAL_DOCUMENTS: usize = 16;

    let cluster = start_subprocess_cluster_with_qwen3_embedding(Qwen3EmbeddingClusterParams {
        agents: AgentConfig::uniform(4, 1),
        buffered_request_timeout: Duration::from_secs(2),
        inference_parameters: InferenceParameters {
            embedding_batch_size: 1,
            enable_embeddings: true,
            ..InferenceParameters::default()
        },
        max_buffered_requests: 4,
    })
    .await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let input_batch: Vec<EmbeddingInputDocument> = (0..TOTAL_DOCUMENTS)
        .map(|index| EmbeddingInputDocument {
            content: format!("Overflow probe document {index}."),
            id: format!("doc-{index}"),
        })
        .collect();

    let stream = inference_client
        .post_generate_embedding_batch(&GenerateEmbeddingBatchParams {
            input_batch,
            normalization_method: EmbeddingNormalizationMethod::None,
        })
        .await?;

    let collected = timeout(Duration::from_secs(15), collect_embedding_results(stream))
        .await
        .map_err(|_| anyhow!("burst-overflow embedding stream did not finish within 15s"))??;

    let overflow_errors: Vec<_> = collected
        .wire_errors
        .iter()
        .filter(|wire_error| wire_error.code == 503)
        .collect();

    assert!(
        !overflow_errors.is_empty(),
        "expected at least one HTTP 503 \"Buffered requests overflow\" envelope, but saw none; wire_errors = {:?}",
        collected.wire_errors,
    );

    for overflow in &overflow_errors {
        assert!(
            overflow.description.contains("Buffered requests overflow"),
            "expected 503 envelope description to mention overflow, got {:?}",
            overflow.description,
        );
    }

    assert!(
        collected.saw_done,
        "stream must terminate cleanly with Done even when some sub-batches overflow",
    );

    assert_eq!(
        collected.embeddings.len() + collected.wire_errors.len(),
        TOTAL_DOCUMENTS,
        "every sub-batch must be accounted for as either a successful embedding or a wire error (503 overflow or 504 timeout): {} embeddings + {} wire errors ({} of which are 503 overflow) ≠ {TOTAL_DOCUMENTS}",
        collected.embeddings.len(),
        collected.wire_errors.len(),
        overflow_errors.len(),
    );

    cluster.shutdown().await?;

    Ok(())
}
