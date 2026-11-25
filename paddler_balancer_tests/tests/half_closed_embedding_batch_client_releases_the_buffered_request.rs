use serde_json::to_value;

use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_cluster_harness::half_closed_client::HalfClosedClient;
use paddler_messaging::api_path::ApiPath;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;

#[tokio::test(flavor = "multi_thread")]
async fn half_closed_embedding_batch_client_releases_the_buffered_request() {
    let mut cluster = Cluster::start_in_process(ClusterParams::without_agents())
        .await
        .expect("a balancer serving text generation and embeddings must start");
    let mut client = HalfClosedClient::post_json_then_half_close(
        cluster.balancer.addresses.inference,
        ApiPath::generate_embedding_batch(),
        &to_value(GenerateEmbeddingBatchParams {
            input_batch: vec![EmbeddingInputDocument {
                content: "Hello world".to_owned(),
                id: "doc-1".to_owned(),
            }],
            normalization_method: EmbeddingNormalizationMethod::None,
        })
        .expect("the embedding batch parameters must serialize"),
    )
    .await
    .expect("the half-closed embedding batch request must be sent");

    cluster
        .wait_for_buffered_request_count(1)
        .await
        .expect("the embedding batch must be buffered while no agent serves embeddings");

    client
        .half_close()
        .await
        .expect("the request must be half-closed");

    cluster.wait_for_buffered_request_count(0).await.expect(
        "the balancer must notice the half-closed client and release the buffered embedding batch",
    );

    drop(client);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
