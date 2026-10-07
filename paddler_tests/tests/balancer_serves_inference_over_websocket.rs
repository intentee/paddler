#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_messaging::inference_client::message::Message as InferenceMessage;
use paddler_messaging::inference_client::response::Response;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_serves_inference_over_websocket() {
    let cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 1))
        .await
        .expect("the cluster must start");

    let mut stream = cluster
        .client_inference
        .continue_from_raw_prompt(
            CancellationToken::new(),
            ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::new(16).unwrap(),
                raw_prompt: "The capital of France is".to_owned(),
            },
        )
        .await
        .expect("the inference request must be accepted");

    let mut token_count: usize = 0;

    while let Some(message_result) = stream.next().await {
        match message_result.expect("the message must be readable") {
            InferenceMessage::Response(envelope) => match envelope.response {
                Response::GeneratedToken(generated_token_result) => {
                    if generated_token_result.is_token() {
                        token_count += 1;
                    }
                }
                Response::Decision(_) | Response::Embedding(_) => {
                    panic!("inference over websocket produced an unexpected response variant")
                }
            },
            InferenceMessage::Error(envelope) => {
                panic!(
                    "inference over websocket failed: code {}, description {:?}",
                    envelope.error.code, envelope.error.description
                )
            }
        }
    }

    assert!(token_count > 0);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
