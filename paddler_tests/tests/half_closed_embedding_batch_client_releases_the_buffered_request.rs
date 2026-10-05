use paddler_inference_parameters::inference_parameters::InferenceParameters;
use paddler_messaging::api_path::ApiPath;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::half_closed_client::HalfClosedClient;
use paddler_test_cluster_harness::raw_agent_socket::RawAgentSocket;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn half_closed_embedding_batch_client_releases_the_buffered_request() {
    let mut cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        desired_state: Some(BalancerDesiredState {
            inference_parameters: InferenceParameters {
                enable_embeddings: true,
                ..InferenceParameters::default()
            },
            ..BalancerDesiredState::default()
        }),
        wait_for_slots_ready: false,
        ..ClusterParams::default()
    })
    .await
    .expect("a balancer serving embeddings must start");
    let mut agent_without_free_slots = RawAgentSocket::connect(
        cluster.balancer.addresses.management,
        "agent-without-free-slots",
    )
    .await
    .expect("the agent connection must be established");

    agent_without_free_slots
        .register()
        .await
        .expect("the agent must register");

    let mut client = HalfClosedClient::post_json_then_half_close(
        cluster.balancer.addresses.inference,
        ApiPath::GENERATE_EMBEDDING_BATCH,
        &GenerateEmbeddingBatchParams {
            input_batch: vec![EmbeddingInputDocument {
                content: "Hello world".to_owned(),
                id: "doc-1".to_owned(),
            }],
            normalization_method: EmbeddingNormalizationMethod::None,
        },
    )
    .await
    .expect("the half-closed embedding batch request must be sent");

    cluster
        .wait_for_buffered_request_count(1)
        .await
        .expect("the embedding batch must be buffered while no agent slot is free");

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
