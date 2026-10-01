#![cfg(feature = "tests_that_use_llms")]

use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_messaging::inference_client::message::Message;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::unending_generation::unending_generation;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

const BAD_GATEWAY: i32 = 502;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_reports_bad_gateway_when_agent_disconnects_mid_generation() {
    let mut cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 1))
        .await
        .expect("the cluster must start");

    let mut stream = cluster
        .continue_from_raw_prompt_stream(CancellationToken::new(), &unending_generation())
        .await
        .expect("the inference request must be accepted");

    stream
        .next()
        .await
        .expect("the request must stream a first token")
        .expect("the message must be readable");

    cluster
        .agents
        .pop()
        .expect("cluster must have one running agent")
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");

    while let Some(message) = stream.next().await {
        if let Message::Error(error_envelope) = message.expect("the message must be readable") {
            assert_eq!(error_envelope.error.code, BAD_GATEWAY);

            cluster
                .shutdown()
                .await
                .expect("the cluster must shut down cleanly");

            return;
        }
    }

    panic!("the stream ended without reporting the lost agent")
}
