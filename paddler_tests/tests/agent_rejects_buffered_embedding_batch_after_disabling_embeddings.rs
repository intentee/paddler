#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_inference_parameters::inference_parameters::InferenceParameters;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::load_qwen3_after_request_is_buffered::load_qwen3_after_request_is_buffered;
use paddler_tests::start_cluster_without_model::start_cluster_without_model;

#[tokio::test(flavor = "multi_thread")]
async fn agent_rejects_buffered_embedding_batch_after_disabling_embeddings() {
    let mut cluster = start_cluster_without_model(
        AgentConfig::uniform(1, 1),
        InferenceParameters {
            enable_embeddings: true,
            ..InferenceParameters::deterministic()
        },
    )
    .await
    .expect("the cluster must start");

    let request = cluster.generate_embedding_batch(
        CancellationToken::new(),
        &GenerateEmbeddingBatchParams {
            input_batch: vec![EmbeddingInputDocument {
                content: "Hello".to_owned(),
                id: "greeting".to_owned(),
            }],
            normalization_method: EmbeddingNormalizationMethod::None,
        },
    );
    let collected = load_qwen3_after_request_is_buffered(&mut cluster, request, false)
        .await
        .expect("the model must load after the request is buffered");

    assert!(collected.embeddings_disabled);
    assert!(collected.embeddings.is_empty());

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
