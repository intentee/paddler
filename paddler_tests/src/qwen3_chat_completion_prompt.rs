use std::time::SystemTime;

use anyhow::Context as _;
use anyhow::Result;
use serde_json::Value;
use serde_json::from_value;

use paddler_agent_text_generation::chat_prompt_render_request::ChatPromptRenderRequest;
use paddler_agent_text_generation::chat_prompt_renderer::ChatPromptRenderer;
use paddler_agent_text_generation::chat_template_renderer::ChatTemplateRenderer;
use paddler_messaging::chat_template_content_form::ChatTemplateContentForm;
use paddler_messaging::chat_template_message::ChatTemplateMessage;
use paddler_messaging::media_marker::MediaMarker;
use paddler_openai_translation::chat_completion_request::ChatCompletionRequest;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;

use crate::agent_inference_runtime_context::agent_inference_runtime_context;
use crate::llama_runtime::LlamaRuntime;

const MEDIA_MARKER: &str = "<media>";

fn qwen3_chat_prompt_renderer() -> Result<ChatPromptRenderer> {
    qwen3_0_6b()
        .cached_path()
        .context("the Qwen3 model must be in the Hugging Face cache")
        .and_then(|model_path| {
            LlamaRuntime::load(&model_path, agent_inference_runtime_context(1))
                .context("the Qwen3 model must load")
        })
        .and_then(|runtime| {
            runtime
                .loaded_llama_model
                .model_metadata
                .embedded_chat_template()
                .context("the Qwen3 model must embed its chat template")
        })
        .and_then(|chat_template| {
            ChatTemplateRenderer::new(chat_template).context("the Qwen3 chat template must compile")
        })
        .map(|chat_template_renderer| ChatPromptRenderer {
            chat_template_renderer,
            token_bos_str: String::new(),
            token_eos_str: String::new(),
            token_nl_str: String::new(),
        })
}

fn chat_template_messages(chat_completion_request: Value) -> Result<Vec<ChatTemplateMessage>> {
    from_value::<ChatCompletionRequest>(chat_completion_request)
        .context("the chat completion request must deserialize")
        .and_then(|request| {
            request
                .translate(SystemTime::now())
                .context("the chat completion request must translate")
        })
        .map(|translated_request| {
            translated_request
                .conversation_history_params
                .conversation_history
                .into_chat_template_conversation(
                    ChatTemplateContentForm::JoinedText,
                    &MediaMarker::new(MEDIA_MARKER.to_owned()),
                )
                .messages
        })
}

pub fn qwen3_chat_completion_prompt(chat_completion_request: Value) -> Result<String> {
    qwen3_chat_prompt_renderer().and_then(|chat_prompt_renderer| {
        chat_template_messages(chat_completion_request).and_then(|messages| {
            chat_prompt_renderer
                .render(ChatPromptRenderRequest {
                    add_generation_prompt: true,
                    enable_thinking: false,
                    messages: &messages,
                    tools: &Value::Array(Vec::new()),
                })
                .context("the Qwen3 chat template must render the conversation")
        })
    })
}
