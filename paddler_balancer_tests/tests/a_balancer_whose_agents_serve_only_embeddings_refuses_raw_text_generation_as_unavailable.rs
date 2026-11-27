#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_client::error::Error as ClientError;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::embedding_cluster_params::EmbeddingClusterParams;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;

#[tokio::test(flavor = "multi_thread")]
async fn a_balancer_whose_agents_serve_only_embeddings_refuses_raw_text_generation_as_unavailable()
{
    let cluster =
        Cluster::start_in_process(EmbeddingClusterParams::default().into_cluster_params())
            .await
            .expect("a cluster serving an embedding model must start");

    let refusal = cluster
        .client_inference
        .post_continue_from_raw_prompt(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::MIN,
                raw_prompt: "Hello".to_owned(),
            },
        )
        .await
        .err()
        .expect("a cluster serving only embeddings must refuse raw text generation");

    assert!(matches!(refusal, ClientError::ServiceUnavailable { .. }));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
