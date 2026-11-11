use serde_json::json;

use paddler_balancer::constants::OPENAI_API_PATH_RESPONSES;
use paddler_tests::release_half_closed_openai_request::release_half_closed_openai_request;

#[tokio::test(flavor = "multi_thread")]
async fn openai_responses_half_closed_client_releases_the_buffered_request() {
    release_half_closed_openai_request(
        OPENAI_API_PATH_RESPONSES,
        &json!({
            "input": "hi",
            "max_output_tokens": 2048,
            "model": "paddler",
            "stream": true,
        }),
    )
    .await
    .expect("the half-closed request must be released");
}
