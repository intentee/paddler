#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use reqwest::StatusCode;
use serde_json::Value;
use serde_json::json;

use paddler_balancer_openai_service::constants::OPENAI_API_PATH_CHAT_COMPLETIONS;
use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::post_openai_chat_completion::post_openai_chat_completion;
use paddler_cluster_harness::start_cluster_with_qwen3_and_context_size::start_cluster_with_qwen3_and_context_size;
use paddler_openai_schema_validator::openai_validator::OpenAIValidator;
use paddler_request_params_fixture::oversized_text::oversized_text;

const SEQUENCE_CONTEXT_SIZE: NonZeroU32 = NonZeroU32::new(256).unwrap();

#[tokio::test(flavor = "multi_thread")]
async fn openai_chat_completion_non_streaming_reports_a_conversation_exceeding_the_context_as_a_bad_request()
 {
    let cluster = start_cluster_with_qwen3_and_context_size(
        vec![AgentConfig::single(1)],
        SEQUENCE_CONTEXT_SIZE,
    )
    .await
    .expect("the cluster must start");

    let response = post_openai_chat_completion(
        cluster
            .balancer
            .compat_openai_url(OPENAI_API_PATH_CHAT_COMPLETIONS)
            .expect("the balancer must serve the OpenAI compatibility service"),
        &json!({
            "model": "test",
            "messages": [{"role": "user", "content": oversized_text()}]
        }),
    )
    .await
    .expect("the balancer must answer the chat completion");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let error_body = response
        .json::<Value>()
        .await
        .expect("the error body must be JSON");

    OpenAIValidator::new()
        .expect("the OpenAI specification must load")
        .validate_error_response(&error_body)
        .expect("the error body must be an OpenAI error");

    assert_eq!(
        error_body.pointer("/error/type").and_then(Value::as_str),
        Some("invalid_request_error")
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
