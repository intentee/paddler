#![cfg(feature = "tests_that_use_llms")]

use serde_json::json;

use crate::qwen3_chat_completion_prompt::qwen3_chat_completion_prompt;

#[test]
fn qwen3_chat_template_renders_the_reasoning_of_an_assistant_turn_in_a_tool_loop() {
    let rendered_prompt = qwen3_chat_completion_prompt(json!({
        "model": "qwen3",
        "messages": [
            { "role": "user", "content": "What is the weather in Paris?" },
            {
                "role": "assistant",
                "content": null,
                "reasoning_content": "I should look the weather up.",
                "tool_calls": [
                    {
                        "id": "call_weather",
                        "type": "function",
                        "function": { "name": "get_weather", "arguments": "{\"location\":\"Paris\"}" }
                    }
                ]
            },
            { "role": "tool", "tool_call_id": "call_weather", "content": "sunny" }
        ]
    }));

    assert_eq!(
        rendered_prompt,
        concat!(
            "<|im_start|>user\n",
            "What is the weather in Paris?",
            "<|im_end|>\n",
            "<|im_start|>assistant\n",
            "<think>\nI should look the weather up.\n</think>\n\n",
            "<tool_call>\n{\"name\": \"get_weather\", \"arguments\": {\"location\": \"Paris\"}}\n</tool_call>",
            "<|im_end|>\n",
            "<|im_start|>user\n",
            "<tool_response>\nsunny\n</tool_response>",
            "<|im_end|>\n",
            "<|im_start|>assistant\n",
            "<think>\n\n</think>\n\n",
        )
    );
}
