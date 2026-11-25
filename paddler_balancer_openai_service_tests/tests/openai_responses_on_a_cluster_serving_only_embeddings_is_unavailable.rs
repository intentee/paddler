#![cfg(feature = "tests_that_use_llms")]

use reqwest::StatusCode;

use paddler_balancer_openai_service::constants::OPENAI_API_PATH_RESPONSES;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::embedding_cluster_params::EmbeddingClusterParams;
use paddler_cluster_harness::minimal_openai_response::minimal_openai_response;

#[tokio::test(flavor = "multi_thread")]
async fn openai_responses_on_a_cluster_serving_only_embeddings_is_unavailable() {
    let cluster =
        Cluster::start_in_process(EmbeddingClusterParams::default().into_cluster_params())
            .await
            .expect("a cluster serving an embedding model must start");

    let response = minimal_openai_response(
        cluster
            .balancer
            .compat_openai_url(OPENAI_API_PATH_RESPONSES)
            .expect("the balancer must serve the OpenAI compatibility service"),
    )
    .await
    .expect("the balancer must answer the response request");

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
