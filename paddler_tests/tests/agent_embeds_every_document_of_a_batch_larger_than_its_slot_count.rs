#![cfg(feature = "tests_that_use_llms")]

use std::collections::BTreeSet;

use tokio_util::sync::CancellationToken;

use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::embedding_cluster_params::EmbeddingClusterParams;
use paddler_tests::numbered_embedding_documents::numbered_embedding_documents;
use paddler_tests::start_embedding_cluster::start_embedding_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn agent_embeds_every_document_of_a_batch_larger_than_its_slot_count() {
    let cluster = start_embedding_cluster(EmbeddingClusterParams {
        agents: vec![AgentConfig::single(4)],

        ..EmbeddingClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let input_batch = numbered_embedding_documents(12);
    let expected_ids: BTreeSet<String> = input_batch
        .iter()
        .map(|document| document.id.clone())
        .collect();

    let collected = cluster
        .generate_embedding_batch(
            CancellationToken::new(),
            &GenerateEmbeddingBatchParams {
                input_batch,
                normalization_method: EmbeddingNormalizationMethod::None,
            },
        )
        .await
        .expect("the embedding batch must be accepted");

    assert_eq!(collected.embeddings.len(), 12);
    assert!(collected.saw_done);
    assert_eq!(collected.failures, Vec::<EmbeddingResult>::new());

    let returned_ids: BTreeSet<String> = collected
        .embeddings
        .iter()
        .map(|produced| produced.embedding.source_document_id.clone())
        .collect();
    assert_eq!(returned_ids, expected_ids);

    let first_dimension = collected.embeddings[0].embedding.embedding.len();

    for produced in &collected.embeddings {
        assert_eq!(produced.embedding.embedding.len(), first_dimension);
    }

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
