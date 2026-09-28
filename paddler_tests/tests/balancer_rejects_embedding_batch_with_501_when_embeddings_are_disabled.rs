use paddler_client::error::Error as ClientError;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;
use tokio_util::sync::CancellationToken;

const NOT_IMPLEMENTED: u16 = 501;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_rejects_embedding_batch_with_501_when_embeddings_are_disabled() {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a cluster without agents must start");

    let rejection = cluster
        .client_inference
        .post_generate_embedding_batch(
            CancellationToken::new(),
            &GenerateEmbeddingBatchParams {
                input_batch: vec![EmbeddingInputDocument {
                    content: "Hello world".to_owned(),
                    id: "doc-1".to_owned(),
                }],
                normalization_method: EmbeddingNormalizationMethod::None,
            },
        )
        .await
        .err()
        .expect("the balancer must reject embedding batches while embeddings are disabled");

    assert!(matches!(
        rejection,
        ClientError::UnexpectedResponseStatus { message, status, .. }
            if status.as_u16() == NOT_IMPLEMENTED
                && message == "Embedding generation is not enabled in the inference parameters"
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
