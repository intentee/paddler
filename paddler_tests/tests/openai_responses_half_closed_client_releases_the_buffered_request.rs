use serde_json::json;

use paddler_balancer::compatibility::openai_service::openai_api_path::OpenAIApiPath;
use paddler_tests::release_half_closed_openai_request::release_half_closed_openai_request;

#[tokio::test(flavor = "multi_thread")]
async fn openai_responses_half_closed_client_releases_the_buffered_request() {
    release_half_closed_openai_request(
        OpenAIApiPath::RESPONSES,
        &json!({
            "input": "hi",
            "max_output_tokens": 2048,
            "model": "paddler",
            "stream": true,
        }),
    )
    .await
    .expect("the OpenAI-compatible responses stream must release its buffered request when the client half-closes");
}
