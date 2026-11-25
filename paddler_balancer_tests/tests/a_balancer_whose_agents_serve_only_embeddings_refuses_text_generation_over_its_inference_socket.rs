#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::embedding_cluster_params::EmbeddingClusterParams;
use paddler_cluster_harness::raw_inference_socket::RawInferenceSocket;
use paddler_messaging::inference_client::message::Message as InferenceClientMessage;

const REFUSED_REQUEST_ID: &str = "refused-request-id";

#[tokio::test(flavor = "multi_thread")]
async fn a_balancer_whose_agents_serve_only_embeddings_refuses_text_generation_over_its_inference_socket()
 {
    let cluster =
        Cluster::start_in_process(EmbeddingClusterParams::default().into_cluster_params())
            .await
            .expect("a cluster serving an embedding model must start");
    let mut socket = RawInferenceSocket::connect(cluster.balancer.addresses.inference)
        .await
        .expect("the raw inference socket must connect");

    socket
        .send_raw_prompt_request(REFUSED_REQUEST_ID, NonZeroU32::MIN)
        .await
        .expect("the request must be sent");

    assert!(matches!(
        socket
            .next_answer_to(REFUSED_REQUEST_ID)
            .await
            .expect("the request must be answered"),
        InferenceClientMessage::Error(envelope) if envelope.error.code == 503
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
