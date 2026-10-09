#![cfg(feature = "tests_that_use_llms")]

use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::embedding_cluster_params::EmbeddingClusterParams;
use paddler_tests::start_embedding_cluster::start_embedding_cluster;

const SLOTS: u16 = 4;

fn embedding_batch(document_count: usize) -> GenerateEmbeddingBatchParams {
    GenerateEmbeddingBatchParams {
        input_batch: (0..document_count)
            .map(|document_index| EmbeddingInputDocument {
                content: format!("Document number {document_index}."),
                id: format!("doc-{document_index}"),
            })
            .collect(),
        normalization_method: EmbeddingNormalizationMethod::None,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_stops_an_embedding_batch_when_the_client_disconnects() {
    let embedding_parameters = EmbeddingParameters::default();
    let documents_in_one_chunk = embedding_parameters.embedding_batch_size.get();
    let mut cluster = start_embedding_cluster(EmbeddingClusterParams {
        agents: vec![AgentConfig::single(SLOTS)],
        embedding_parameters,
        ..EmbeddingClusterParams::default()
    })
    .await
    .expect("the cluster must start");
    let agent_id = cluster
        .agent_ids
        .first()
        .expect("the cluster must have an agent")
        .clone();
    let mut embedding_stream = cluster
        .client_inference
        .post_generate_embedding_batch(
            CancellationToken::new(),
            &embedding_batch(documents_in_one_chunk),
        )
        .await
        .expect("the embedding batch must be accepted");

    embedding_stream
        .next()
        .await
        .expect("the agent must produce a first embedding")
        .expect("the first embedding must be readable");
    drop(embedding_stream);

    cluster
        .wait_for_slots_processing(&agent_id, 0)
        .await
        .expect("the agent must stop the abandoned embedding batch");

    let collected = cluster
        .generate_embedding_batch(
            CancellationToken::new(),
            &embedding_batch(usize::from(SLOTS)),
        )
        .await
        .expect("an embedding batch after the abandoned one must be accepted");

    assert_eq!(collected.embeddings.len(), usize::from(SLOTS));
    assert!(collected.saw_done);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
