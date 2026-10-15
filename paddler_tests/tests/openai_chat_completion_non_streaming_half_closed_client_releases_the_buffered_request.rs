use serde_json::json;

use paddler_balancer::compatibility::openai_service::openai_api_path::OpenAIApiPath;
use paddler_tests::release_half_closed_openai_request::release_half_closed_openai_request;

#[tokio::test(flavor = "multi_thread")]
async fn openai_chat_completion_non_streaming_half_closed_client_releases_the_buffered_request() {
    release_half_closed_openai_request(
        OpenAIApiPath::CHAT_COMPLETIONS,
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
