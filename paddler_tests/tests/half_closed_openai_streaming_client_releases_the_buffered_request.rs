use serde_json::json;

use paddler_balancer::compatibility::openai_service::openai_api_path::OpenAIApiPath;
use paddler_tests::release_half_closed_openai_request::release_half_closed_openai_request;

#[tokio::test(flavor = "multi_thread")]
async fn half_closed_openai_streaming_client_releases_the_buffered_request() {
    release_half_closed_openai_request(
        OpenAIApiPath::CHAT_COMPLETIONS,
        &json!({
            "max_completion_tokens": 2048,
            "messages": [{"role": "user", "content": "hi"}],
            "model": "paddler",
            "stream": true,
        }),
    )
    .await
    .expect("the OpenAI-compatible SSE stream must release its buffered request when the client half-closes");
}
