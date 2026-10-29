#![cfg(feature = "tests_that_use_llms")]

use serde_json::json;

use paddler_tests::qwen3_chat_completion_prompt::qwen3_chat_completion_prompt;

#[test]
fn qwen3_chat_template_renders_a_tool_call_history_as_tool_call_and_tool_response_blocks() {
    let rendered_prompt = qwen3_chat_completion_prompt(json!({
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
    .expect("the Qwen3 chat template must render the tool call history");

    assert_eq!(
        rendered_prompt,
        concat!(
            "<|im_start|>user\n",
            "What is the weather and the time in Paris?",
            "<|im_end|>\n",
            "<|im_start|>assistant\n",
            "<tool_call>\n{\"name\": \"get_weather\", \"arguments\": {\"location\": \"Paris\"}}\n</tool_call>\n",
            "<tool_call>\n{\"name\": \"get_time\", \"arguments\": {\"location\": \"Paris\"}}\n</tool_call>",
            "<|im_end|>\n",
            "<|im_start|>user\n",
            "<tool_response>\nsunny\n</tool_response>\n",
            "<tool_response>\nnoon\n</tool_response>",
            "<|im_end|>\n",
            "<|im_start|>assistant\n",
            "<think>\n\n</think>\n\n",
        )
    );
}
