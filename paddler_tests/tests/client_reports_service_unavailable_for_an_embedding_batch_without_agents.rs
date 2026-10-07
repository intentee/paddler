use tokio_util::sync::CancellationToken;

use paddler_client::error::Error;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn client_reports_service_unavailable_for_an_embedding_batch_without_agents() {
    let cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        desired_state: ClusterDesiredState::Apply(Box::new(BalancerDesiredState::unconfigured(
            InferenceMode::Embeddings,
        ))),
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
        .expect("the balancer must reject an embedding batch while no agent can serve it");

    assert!(matches!(
        rejection,
        Error::ServiceUnavailable { message, .. } if message == "No agents are currently connected"
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
