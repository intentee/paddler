#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use futures_util::StreamExt as _;
use paddler_messaging::inference_client::message::Message as InferenceClientMessage;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;
use tokio_util::sync::CancellationToken;

const MAX_TOKENS_THAT_OUTLAST_THE_SHUTDOWN: NonZeroU32 = NonZeroU32::new(2048).unwrap();

#[tokio::test(flavor = "multi_thread")]
async fn balancer_ends_an_in_flight_generation_with_a_shutdown_error() {
    let cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 1))
        .await
        .expect("a single-slot qwen3 cluster must start");
    let mut stream = cluster
        .client_inference
        .post_continue_from_raw_prompt(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: MAX_TOKENS_THAT_OUTLAST_THE_SHUTDOWN,
                raw_prompt: "Tell me a long story about a lighthouse keeper.".to_owned(),
            },
        )
        .await
        .expect("the generation must start");

    stream
        .next()
        .await
        .expect("the generation must stream its first message")
        .expect("the first message must be readable");

    let Cluster {
        agents, balancer, ..
    } = cluster;
    let balancer_shutdown = tokio::spawn(balancer.shutdown());
    let mut shutdown_error_codes = Vec::new();

    while let Some(message) = stream.next().await {
        if let InferenceClientMessage::Error(envelope) =
            message.expect("every streamed message must be readable")
        {
            shutdown_error_codes.push(envelope.error.code);
        }
    }

    assert_eq!(shutdown_error_codes, vec![503]);

    balancer_shutdown
        .await
        .expect("the balancer shutdown task must not panic")
        .expect("the balancer must shut down cleanly");

    for agent in agents {
        agent
            .shutdown()
            .await
            .expect("the agent must shut down cleanly");
    }
}
