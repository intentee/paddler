use paddler_tests::release_half_closed_openai_request::release_half_closed_openai_request;
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn half_closed_openai_non_streaming_client_releases_the_buffered_request() {
    release_half_closed_openai_request(
        "/v1/chat/completions",
        &json!({
            "max_completion_tokens": 2048,
            "messages": [{"role": "user", "content": "hi"}],
            "model": "paddler",
            "stream": false,
        }),
    )
    .await
    .expect("a non-streaming OpenAI-compatible request buffers its response inside the handler, so the balancer must still release it when the client half-closes");
}
