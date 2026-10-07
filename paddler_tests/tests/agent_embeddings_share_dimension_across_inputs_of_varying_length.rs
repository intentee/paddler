#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::embedding_cluster_params::EmbeddingClusterParams;
use paddler_tests::start_embedding_cluster::start_embedding_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn agent_embeddings_share_dimension_across_inputs_of_varying_length() {
    let cluster = start_embedding_cluster(EmbeddingClusterParams {
        agents: vec![AgentConfig::single(1)],

        ..EmbeddingClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let collected = cluster
        .generate_embedding_batch(CancellationToken::new(), &GenerateEmbeddingBatchParams {
            input_batch: vec![
                EmbeddingInputDocument {
                    content: "Hello".to_owned(),
                    id: "doc-short".to_owned(),
                },
                EmbeddingInputDocument {
                    content: "The quick brown fox jumped over the lazy dog.".to_owned(),
                    id: "doc-medium".to_owned(),
                },
                EmbeddingInputDocument {
                    content: "Rust is a systems programming language focused on safety, speed, and concurrency. It achieves memory safety without garbage collection.".to_owned(),
                    id: "doc-long".to_owned(),
                },
            ],
            normalization_method: EmbeddingNormalizationMethod::None,
        })
        .await.expect("the embedding batch must be accepted");

    assert_eq!(collected.embeddings.len(), 3);
    assert!(collected.saw_done);

    let first_dimension = collected.embeddings[0].embedding.embedding.len();

    assert!(first_dimension > 0, "embedding dimension must be positive");

    for produced in &collected.embeddings {
        assert_eq!(
            produced.embedding.embedding.len(),
            first_dimension,
            "all embeddings must share dimension; {} has {} instead of {}",
            produced.embedding.source_document_id,
            produced.embedding.embedding.len(),
            first_dimension
        );
    }

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
