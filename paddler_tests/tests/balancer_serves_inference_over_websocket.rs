#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_messaging::inference_client::message::Message as InferenceMessage;
use paddler_messaging::inference_client::response::Response;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_messaging::streamable_result::StreamableResult as _;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_serves_inference_over_websocket() {
    let cluster = start_cluster_with_qwen3(AgentConfig::uniform(1, 1))
        .await
        .expect("the cluster must start");

    let stream = cluster
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

    let messages: Vec<InferenceMessage> = stream
        .map(|message_result| message_result.expect("the message must be readable"))
        .collect()
        .await;

    assert!(messages.iter().all(|message| matches!(
        message,
        InferenceMessage::Response(ResponseEnvelope {
            response: Response::GeneratedToken(_),
            ..
        })
    )));
    assert!(messages.iter().any(|message| matches!(
        message,
        InferenceMessage::Response(ResponseEnvelope {
            response: Response::GeneratedToken(token_result),
            ..
        }) if token_result.is_token()
    )));
    assert!(matches!(
        messages.last(),
        Some(InferenceMessage::Response(ResponseEnvelope {
            response: Response::GeneratedToken(token_result),
            ..
        })) if token_result.is_done()
    ));

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
