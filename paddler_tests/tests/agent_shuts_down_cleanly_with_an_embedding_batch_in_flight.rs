#![cfg(feature = "tests_that_use_llms")]

use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::embedding_cluster_params::EmbeddingClusterParams;
use paddler_tests::start_embedding_cluster::start_embedding_cluster;

const DOCUMENTS_OUTLASTING_A_SHUTDOWN: usize = 512;

#[tokio::test(flavor = "multi_thread")]
async fn agent_shuts_down_cleanly_with_an_embedding_batch_in_flight() {
    let mut cluster = start_embedding_cluster(EmbeddingClusterParams {
        agents: vec![AgentConfig::single(1)],
        ..EmbeddingClusterParams::default()
    })
    .await
    .expect("the cluster must start");
    let mut embedding_stream = cluster
        .client_inference
        .post_generate_embedding_batch(
            CancellationToken::new(),
            &GenerateEmbeddingBatchParams {
                input_batch: (0..DOCUMENTS_OUTLASTING_A_SHUTDOWN)
                    .map(|index| EmbeddingInputDocument {
                        content: format!("Document number {index}."),
                        id: format!("doc-{index}"),
                    })
                    .collect(),
                normalization_method: EmbeddingNormalizationMethod::None,
            },
        )
        .await
        .expect("the embedding batch must be accepted");

    embedding_stream
        .next()
        .await
        .expect("the embedding batch must produce its first embedding")
        .expect("the first embedding must be readable");

    cluster
        .agents
        .pop()
        .expect("the cluster must have an agent")
        .shutdown()
        .await
        .expect("the agent must shut down cleanly with an embedding batch in flight");

    drop(embedding_stream);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
