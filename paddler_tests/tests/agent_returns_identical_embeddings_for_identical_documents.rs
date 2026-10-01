#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_inference_parameters::inference_parameters::InferenceParameters;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::embedding_cluster_params::EmbeddingClusterParams;
use paddler_tests::start_embedding_cluster::start_embedding_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn agent_returns_identical_embeddings_for_identical_documents() {
    let cluster = start_embedding_cluster(EmbeddingClusterParams {
        agents: vec![AgentConfig::single(1)],
        inference_parameters: InferenceParameters {
            enable_embeddings: true,
            ..InferenceParameters::deterministic()
        },
        ..EmbeddingClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let repeated_content = "Deterministic embedding output test.";

    let collected = cluster
        .generate_embedding_batch(
            CancellationToken::new(),
            &GenerateEmbeddingBatchParams {
                input_batch: vec![
                    EmbeddingInputDocument {
                        content: repeated_content.to_owned(),
                        id: "doc-first".to_owned(),
                    },
                    EmbeddingInputDocument {
                        content: repeated_content.to_owned(),
                        id: "doc-second".to_owned(),
                    },
                ],
                normalization_method: EmbeddingNormalizationMethod::None,
            },
        )
        .await
        .expect("the embedding batch must be accepted");

    assert_eq!(collected.embeddings.len(), 2);
    assert!(collected.saw_done);

    let first = collected
        .embeddings
        .iter()
        .find(|produced| produced.embedding.source_document_id == "doc-first")
        .expect("first embedding missing");

    let second = collected
        .embeddings
        .iter()
        .find(|produced| produced.embedding.source_document_id == "doc-second")
        .expect("second embedding missing");

    assert_eq!(
        first.embedding.embedding, second.embedding.embedding,
        "identical documents must produce identical embedding vectors"
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
