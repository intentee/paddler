#![cfg(feature = "tests_that_use_llms")]

use std::time::SystemTime;

use serde_json::Value;
use serde_json::from_value;

use paddler_agent_text_generation::chat_prompt_render_request::ChatPromptRenderRequest;
use paddler_agent_text_generation::chat_prompt_renderer::ChatPromptRenderer;
use paddler_agent_text_generation::chat_template_renderer::ChatTemplateRenderer;
use paddler_messaging::chat_template_content_form::ChatTemplateContentForm;
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
    let chat_prompt_renderer = ChatPromptRenderer {
        chat_template_renderer: ChatTemplateRenderer::new(
            runtime
                .loaded_llama_model
                .model_metadata
                .embedded_chat_template()
                .expect("the Qwen3 model must embed its chat template"),
        )
        .expect("the Qwen3 chat template must compile"),
        token_bos_str: String::new(),
        token_eos_str: String::new(),
        token_nl_str: String::new(),
    };
    let messages = from_value::<ChatCompletionRequest>(chat_completion_request)
        .expect("the chat completion request must deserialize")
        .translate(SystemTime::now())
        .expect("the chat completion request must translate")
        .conversation_history_params
        .conversation_history
        .into_chat_template_conversation(
            ChatTemplateContentForm::JoinedText,
            &MediaMarker::new(MEDIA_MARKER.to_owned()),
        )
        .messages;

    chat_prompt_renderer
        .render(ChatPromptRenderRequest {
            add_generation_prompt: true,
            enable_thinking: false,
            messages: &messages,
            tools: &Value::Array(Vec::new()),
        })
        .expect("the Qwen3 chat template must render the conversation")
}
