#![cfg(feature = "tests_that_use_llms")]

use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::embedding_cluster_params::EmbeddingClusterParams;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_request_params_fixture::numbered_embedding_documents::numbered_embedding_documents;

const DOCUMENTS_OUTLASTING_A_SHUTDOWN: usize = 512;

#[tokio::test(flavor = "multi_thread")]
async fn embedding_scheduler_stops_mid_batch_when_the_agent_shuts_down() {
    let mut cluster = Cluster::start_in_process(
        EmbeddingClusterParams {
            agents: vec![AgentConfig::single(1)],
            ..EmbeddingClusterParams::default()
        }
        .into_cluster_params(),
    )
    .await
    .expect("the cluster must start");
    let mut embedding_stream = cluster
        .client_inference
        .post_generate_embedding_batch(
            CancellationToken::new(),
            &GenerateEmbeddingBatchParams {
                input_batch: numbered_embedding_documents(DOCUMENTS_OUTLASTING_A_SHUTDOWN),
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
