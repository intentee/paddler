use paddler_tests::release_half_closed_openai_request::release_half_closed_openai_request;
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn half_closed_openai_responses_client_releases_the_buffered_request() {
    release_half_closed_openai_request(
        "/v1/responses",
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
