#![cfg(feature = "tests_that_use_llms")]

use serde_json::json;

use paddler_openai_response_format_validator::openai_validator::OpenAIValidator;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_tests::start_cluster_with_qwen3::start_cluster_with_qwen3;

#[tokio::test(flavor = "multi_thread")]
async fn openai_chat_completion_non_streaming_conforms_to_schema() {
    let validator = OpenAIValidator::new().expect("the OpenAI schemas must load");
    let cluster = start_cluster_with_qwen3(vec![AgentConfig::single(1)])
        .await
        .expect("the cluster must start");

    let request = json!({
        "model": "qwen3-test",
        "messages": [{"role": "user", "content": "Say hello."}],
        "max_completion_tokens": 200,
        "stream": false
    });

    validator
        .validate_chat_completion_request(&request)
        .expect("the payload must conform to the OpenAI schema");

    let response = cluster
        .openai_chat_completion_non_streaming(&request)
        .await
        .expect("the OpenAI chat completion must succeed");

    validator
        .validate_chat_completion_response(&response)
        .expect("the payload must conform to the OpenAI schema");

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
