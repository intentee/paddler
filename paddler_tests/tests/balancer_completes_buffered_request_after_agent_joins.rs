#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_messaging::inference_client::message::Message;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_completes_buffered_request_after_agent_joins() {
    let mut cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        max_buffered_requests: 1,
        desired_state: Some(qwen3_0_6b().into_desired_state()),
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    let mut stream = cluster
        .continue_from_raw_prompt_stream(
            CancellationToken::new(),
            &ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::new(10).unwrap(),
                raw_prompt: "Hello".to_owned(),
            },
        )
        .await
        .expect("the inference request must be accepted");

    cluster
        .wait_for_buffered_request_count(1)
        .await
        .expect("balancer should buffer the request before any agent joins");

    cluster
        .spawn_additional_agent(&AgentConfig {
            name: "buffered-agent".to_owned(),
            slot_count: 4,
        })
        .expect("the additional agent must start");

    let message = stream
        .next()
        .await
        .expect("inference stream must yield a message after agent joins")
        .expect("the message must be readable");

    match message {
        Message::Response(_) => {}
        Message::Error(envelope) => {
            panic!(
                "expected a successful response, got error code {}: {}",
                envelope.error.code, envelope.error.description
            );
        }
        Message::Notification(_) => {
            panic!("unexpected token-generation-mode notification");
        }
    }

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
