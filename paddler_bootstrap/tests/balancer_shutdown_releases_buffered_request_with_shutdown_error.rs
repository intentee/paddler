use std::time::Duration;

use paddler_bootstrap::balancer_runner::BalancerRunner;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use tokio_util::sync::CancellationToken;
use trzcina::ServiceShutdownOptions;

use crate::ephemeral_balancer_runner_params::ephemeral_balancer_runner_params;

#[tokio::test]
async fn balancer_shutdown_releases_buffered_request_with_shutdown_error() {
    let mut params = ephemeral_balancer_runner_params(CancellationToken::new());

    params.buffered_request_timeout = Duration::from_mins(1);
    params.shutdown_options = ServiceShutdownOptions {
        cooperative_deadline: Duration::from_secs(2),
        abort_deadline: Duration::from_secs(2),
    };

    let mut runner = BalancerRunner::start(params)
        .await
        .expect("a runner on ephemeral ports must start");
    let held_response = reqwest::Client::new()
        .post(format!(
            "http://{}/api/v1/continue_from_raw_prompt",
            runner.addresses.inference
        ))
        .json(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 10,
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
        .expect("the balancer must shut down before the deadline");

    assert!(
        body.contains("shutting down"),
        "the buffered request must be released with a shutdown error, got body: {body:?}"
    );
}
