#![cfg(feature = "tests_that_use_llms")]

use std::time::SystemTime;

use llama_cpp_bindings::chat_assistant_turn::ChatAssistantTurn;
use llama_cpp_bindings::chat_session::ChatSession;
use llama_cpp_bindings::chat_session_options::ChatSessionOptions;
use llama_cpp_bindings::chat_templates::ChatTemplates;
use llama_cpp_bindings::chat_tool_choice::ChatToolChoice;
use serde_json::Value;
use serde_json::from_value;

use paddler_messaging::media_marker::MediaMarker;
use paddler_openai_translation::chat_completion_request::ChatCompletionRequest;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::llama_runtime::LlamaRuntime;

const MEDIA_MARKER: &str = "<media>";

pub fn qwen3_chat_completion_prompt(chat_completion_request: Value) -> String {
    let runtime = LlamaRuntime::load(
        &qwen3_0_6b()
            .cached_path()
            .expect("the Qwen3 model must be in the Hugging Face cache"),
        agent_inference_runtime_context(1),
    )
    .expect("the Qwen3 model must load");
    let model = &runtime.loaded_llama_model.model;
    let messages = from_value::<ChatCompletionRequest>(chat_completion_request)
        .expect("the chat completion request must deserialize")
        .translate(SystemTime::now())
        .expect("the chat completion request must translate")
        .conversation_history_params
        .conversation_history
        .into_chat_template_conversation(&MediaMarker::new(MEDIA_MARKER.to_owned()))
        .messages;

    ChatSession::new(
        model,
        &ChatTemplates::new(model).expect("the Qwen3 chat template must compile"),
        &messages,
        &[],
        &ChatSessionOptions {
            assistant_turn: ChatAssistantTurn::New,
            enable_thinking: false,
            parallel_tool_calls: false,
            tool_choice: ChatToolChoice::None,
        },
    )
    .expect("the Qwen3 chat template must render the conversation")
    .prompt()
    .expect("the rendered Qwen3 prompt must be readable")
}
