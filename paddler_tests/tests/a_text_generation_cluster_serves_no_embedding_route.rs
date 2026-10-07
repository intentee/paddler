use http::StatusCode;
use tokio_util::sync::CancellationToken;

use paddler_client::error::Error as ClientError;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn a_text_generation_cluster_serves_no_embedding_route() {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a balancer serving text generation must start");

    let embedding_rejection = cluster
        .client_inference
        .post_generate_embedding_batch(
            CancellationToken::new(),
            &GenerateEmbeddingBatchParams {
                input_batch: vec![EmbeddingInputDocument {
                    content: "hello".to_owned(),
                    id: "document".to_owned(),
                }],
                normalization_method: EmbeddingNormalizationMethod::None,
            },
        )
        .await
        .err()
        .expect("a text generation cluster must not serve the embedding route");

    assert!(matches!(
        embedding_rejection,
        ClientError::UnexpectedResponseStatus { status, .. } if status == StatusCode::NOT_FOUND
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
