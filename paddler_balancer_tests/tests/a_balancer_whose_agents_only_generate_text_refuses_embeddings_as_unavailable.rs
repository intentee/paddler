#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_client::error::Error as ClientError;
use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::start_cluster_with_qwen3::start_cluster_with_qwen3;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;

#[tokio::test(flavor = "multi_thread")]
async fn a_balancer_whose_agents_only_generate_text_refuses_embeddings_as_unavailable() {
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)])
        .await
        .expect("a cluster serving a text generation model must start");

    let refusal = cluster
        .client_inference
        .post_generate_embedding_batch(
            CancellationToken::new(),
            &GenerateEmbeddingBatchParams {
                input_batch: vec![EmbeddingInputDocument {
                    content: "the quick brown fox".to_owned(),
                    id: "document".to_owned(),
                }],
                normalization_method: EmbeddingNormalizationMethod::None,
            },
        )
        .await
        .err()
        .expect("a cluster that only generates text must refuse embeddings");

    assert!(matches!(refusal, ClientError::ServiceUnavailable { .. }));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
