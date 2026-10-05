use std::num::NonZeroU32;

use reqwest::Client;
use serde_json::from_str;
use tokio_util::sync::CancellationToken;

use paddler_bootstrap::balancer_runner::BalancerRunner;
use paddler_messaging::api_path::ApiPath;
use paddler_messaging::inference_client::message::Message as OutgoingMessage;
use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
use paddler_messaging::jsonrpc::error_envelope::ErrorEnvelope;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;

use crate::ephemeral_balancer_runner_params::ephemeral_balancer_runner_params;

#[tokio::test]
async fn balancer_shutdown_releases_buffered_request_with_shutdown_error() {
    let runner = BalancerRunner::start(ephemeral_balancer_runner_params(CancellationToken::new()))
        .await
        .expect("a runner on ephemeral ports must start");
    let held_response = Client::new()
        .post(format!(
            "http://{}{}",
            runner.addresses.inference,
            ApiPath::CONTINUE_FROM_RAW_PROMPT
        ))
        .json(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: NonZeroU32::new(10).unwrap(),
            raw_prompt: "hold the connection open during shutdown".to_owned(),
        })
        .send()
        .await
        .expect("the buffered request must receive its response headers");

    runner.cancel();

    let body = held_response
        .text()
        .await
        .expect("the held response body must be readable after shutdown");

    runner
        .wait_for_completion()
        .await
        .expect("the balancer must shut down once it has released the buffered request");

    assert!(matches!(
        from_str::<OutgoingMessage>(body.strip_suffix('\n').expect("the body must be one NDJSON line"))
            .expect("the body must be an inference message"),
        OutgoingMessage::Error(ErrorEnvelope {
            error: JsonRpcError {
                code: 503,
                description,
            },
            ..
        }) if description == "balancer is shutting down"
    ));
}
