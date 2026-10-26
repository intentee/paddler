#![cfg(feature = "tests_that_use_llms")]

use std::time::SystemTime;

use serde_json::Value;
use serde_json::from_value;
use serde_json::json;

use paddler_agent_text_generation::chat_prompt_render_request::ChatPromptRenderRequest;
use paddler_agent_text_generation::chat_prompt_renderer::ChatPromptRenderer;
use paddler_agent_text_generation::chat_template_renderer::ChatTemplateRenderer;
use paddler_messaging::media_marker::MediaMarker;
use paddler_openai_translation::chat_completion_request::ChatCompletionRequest;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::llama_runtime::LlamaRuntime;

#[test]
fn qwen3_chat_template_renders_a_tool_call_history_as_tool_call_and_tool_response_blocks() {
    let runtime = LlamaRuntime::load(
        &qwen3_0_6b()
            .cached_path()
            .expect("the Qwen3 model must be in the Hugging Face cache"),
        agent_inference_runtime_context(1),
    )
    .expect("the Qwen3 model must load");
    let media_marker = MediaMarker::new("<media>".to_owned());
    let chat_prompt_renderer = ChatPromptRenderer {
        chat_template_renderer: ChatTemplateRenderer::new(
            runtime
                .loaded_llama_model
                .model_metadata
                .embedded_chat_template()
                .expect("the Qwen3 model must embed its chat template"),
        )
        .expect("the Qwen3 chat template must compile"),
        media_marker: MediaMarker::new("<media>".to_owned()),
        token_bos_str: String::new(),
        token_eos_str: String::new(),
        token_nl_str: String::new(),
    };
    let conversation_history_params = from_value::<ChatCompletionRequest>(json!({
        "model": "qwen3",
        "messages": [
            { "role": "user", "content": "What is the weather and the time in Paris?" },
            {
                "role": "assistant",
                "content": null,
                "tool_calls": [
                    {
                        "id": "call_weather",
                        "type": "function",
                        "function": { "name": "get_weather", "arguments": "{\"location\":\"Paris\"}" }
                    },
                    {
                        "id": "call_time",
                        "type": "function",
                        "function": { "name": "get_time", "arguments": "{\"location\":\"Paris\"}" }
                    }
                ]
            },
            { "role": "tool", "tool_call_id": "call_weather", "content": "sunny" },
            { "role": "tool", "tool_call_id": "call_time", "content": "noon" }
        ]
    }))
    .expect("the chat completion request must deserialize")
    .translate(SystemTime::now())
    .expect("the chat completion request must translate")
    .conversation_history_params;
    let rendered_prompt = chat_prompt_renderer
        .render(ChatPromptRenderRequest {
            add_generation_prompt: true,
            enable_thinking: false,
            messages: &conversation_history_params
                .conversation_history
                .into_chat_template_conversation(&media_marker)
                .messages,
            tools: &Value::Array(Vec::new()),
        })
        .expect("the Qwen3 chat template must render the tool call history");

    assert!(rendered_prompt.contains(concat!(
        "<|im_start|>assistant\n",
        "<tool_call>\n{\"name\": \"get_weather\", \"arguments\": {\"location\": \"Paris\"}}\n</tool_call>\n",
        "<tool_call>\n{\"name\": \"get_time\", \"arguments\": {\"location\": \"Paris\"}}\n</tool_call>",
        "<|im_end|>\n",
        "<|im_start|>user\n",
        "<tool_response>\nsunny\n</tool_response>\n",
        "<tool_response>\nnoon\n</tool_response>",
        "<|im_end|>\n",
    )));
}
