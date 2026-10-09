use tokio_util::sync::CancellationToken;

use paddler_client::error::Error as ClientError;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_tests::cluster_without_agents_serving::cluster_without_agents_serving;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn a_cluster_serving_text_generation_refuses_embedding_batches_as_unavailable() {
    let cluster = start_cluster(cluster_without_agents_serving(
        InferenceMode::TextGeneration,
    ))
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
        .expect("a text generation cluster must refuse an embedding batch");

    assert!(matches!(
        embedding_rejection,
        ClientError::ServiceUnavailable { message, .. }
            if message == "The cluster serves TextGeneration, not Embeddings"
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
