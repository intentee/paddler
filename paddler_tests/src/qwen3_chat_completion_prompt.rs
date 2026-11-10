use std::time::SystemTime;

use anyhow::Context as _;
use anyhow::Result;
use llama_cpp_bindings::chat_assistant_turn::ChatAssistantTurn;
use llama_cpp_bindings::chat_session::ChatSession;
use llama_cpp_bindings::chat_session_options::ChatSessionOptions;
use llama_cpp_bindings::chat_templates::ChatTemplates;
use llama_cpp_bindings::chat_tool_choice::ChatToolChoice;
use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings_types::chat_message::ChatMessage;
use serde_json::Value;
use serde_json::from_value;

use paddler_messaging::media_marker::MediaMarker;
use paddler_openai_translation::chat_completion_request::ChatCompletionRequest;
use paddler_openai_translation::translated_chat_completion_request::TranslatedChatCompletionRequest;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;

use crate::agent_inference_runtime_context::agent_inference_runtime_context;
use crate::llama_runtime::LlamaRuntime;

const MEDIA_MARKER: &str = "<media>";

fn conversation_messages(chat_completion_request: Value) -> Result<Vec<ChatMessage>> {
    from_value::<ChatCompletionRequest>(chat_completion_request)
        .context("the chat completion request must deserialize")
        .and_then(|chat_completion_request| {
            chat_completion_request
                .translate(SystemTime::now())
                .context("the chat completion request must translate")
        })
        .map(
            |TranslatedChatCompletionRequest {
                 conversation_history_params,
                 ..
             }| {
                conversation_history_params
                    .conversation_history
                    .into_chat_template_conversation(&MediaMarker::new(MEDIA_MARKER.to_owned()))
                    .messages
            },
        )
}

fn rendered_prompt(model: &LlamaModel, chat_completion_request: Value) -> Result<String> {
    conversation_messages(chat_completion_request)
        .and_then(|messages| {
            ChatTemplates::new(model)
                .context("the Qwen3 chat template must compile")
                .and_then(|chat_templates| {
                    ChatSession::new(
                        model,
                        &chat_templates,
                        &messages,
                        &[],
                        &ChatSessionOptions {
                            assistant_turn: ChatAssistantTurn::New,
                            enable_thinking: false,
                            parallel_tool_calls: false,
                            tool_choice: ChatToolChoice::None,
                        },
                    )
                    .context("the Qwen3 chat template must render the conversation")
                })
        })
        .and_then(|chat_session| {
            chat_session
                .prompt()
                .context("the rendered Qwen3 prompt must be readable")
        })
}

/// # Errors
///
/// Returns an error when the Qwen3 model is not in the Hugging Face cache or cannot load, when the chat completion request does not deserialize or translate, and when the Qwen3 chat template cannot compile, render, or read back the conversation.
pub fn qwen3_chat_completion_prompt(chat_completion_request: Value) -> Result<String> {
    qwen3_0_6b()
        .cached_path()
        .context("the Qwen3 model must be in the Hugging Face cache")
        .and_then(|model_path| {
            LlamaRuntime::load(&model_path, agent_inference_runtime_context(1))
                .context("the Qwen3 model must load")
        })
        .and_then(
            |LlamaRuntime {
                 loaded_llama_model, ..
             }| rendered_prompt(&loaded_llama_model.model, chat_completion_request),
        )
}
