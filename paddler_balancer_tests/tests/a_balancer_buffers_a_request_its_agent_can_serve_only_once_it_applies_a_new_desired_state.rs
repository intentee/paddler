#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_balancer_desired_state_fixture::qwen3_desired_state::qwen3_desired_state;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::embedding_cluster_params::EmbeddingClusterParams;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;

#[tokio::test(flavor = "multi_thread")]
async fn a_balancer_buffers_a_request_its_agent_can_serve_only_once_it_applies_a_new_desired_state()
{
    let cluster =
        Cluster::start_in_process(EmbeddingClusterParams::default().into_cluster_params())
            .await
            .expect("a cluster serving an embedding model must start");

    cluster
        .client_management
        .put_balancer_desired_state(CancellationToken::new(), &qwen3_desired_state())
        .await
        .expect("the balancer must accept the text generation model");

    let generated = cluster
        .continue_from_raw_prompt(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::MIN,
                raw_prompt: "Hello".to_owned(),
            },
        )
        .await
        .expect("the raw prompt must wait until the agent applies the text generation model");

    assert_eq!(
        generated
            .summary()
            .expect("the generation must finish with a summary")
            .usage
            .completion_tokens(),
        1
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
