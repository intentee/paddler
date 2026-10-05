use std::num::NonZeroU32;
use std::time::Duration;

use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_messaging::inference_client::message::Message;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::start_cluster::start_cluster;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_returns_503_when_request_buffering_disabled() {
    let mut cluster = start_cluster(ClusterParams {
        agents: Vec::new(),
        wait_for_slots_ready: false,
        buffered_request_timeout: Duration::from_millis(50),
        max_buffered_requests: 0,
        ..ClusterParams::default()
    })
    .await
    .expect("the cluster must start");

    cluster
        .spawn_additional_agent(&AgentConfig {
            name: "buffer-disabled-agent".to_owned(),
            slot_count: 2,
        })
        .expect("the additional agent must start");

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

    let message = stream
        .next()
        .await
        .expect("inference stream must yield a message")
        .expect("the message must be readable");

    match message {
        Message::Error(envelope) => {
            assert_eq!(envelope.error.code, 503);
        }
        Message::Response(_) => {
            panic!("expected buffer overflow error, got success");
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
