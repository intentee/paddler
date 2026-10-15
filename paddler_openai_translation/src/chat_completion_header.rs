use paddler_messaging::generation_summary::GenerationSummary;

use crate::chat_completion::ChatCompletion;
use crate::chat_completion_finish_reason::chat_completion_finish_reason;
use crate::chat_completion_usage::ChatCompletionUsage;
use crate::completes_generation::CompletesGeneration;
use crate::generated_output::GeneratedOutput;

#[derive(Clone)]
pub struct ChatCompletionHeader {
    pub created: u64,
    pub model: String,
}

impl CompletesGeneration for ChatCompletionHeader {
    type Completion = ChatCompletion;

    fn complete(
        &self,
        request_id: &str,
        GeneratedOutput {
            content,
            tool_calls,
            ..
        }: GeneratedOutput,
        GenerationSummary { finish, usage }: &GenerationSummary,
    ) -> ChatCompletion {
        ChatCompletion {
            content,
            created: self.created,
            finish_reason: chat_completion_finish_reason(*finish, !tool_calls.is_empty()),
            id: request_id.to_owned(),
            model: self.model.clone(),
            tool_calls,
            usage: ChatCompletionUsage(*usage),
        }
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings_types::ParsedToolCall;
    use llama_cpp_bindings_types::TokenUsage;
    use llama_cpp_bindings_types::ToolCallArguments;
    use serde_json::json;
    use serde_json::to_string;

    use paddler_messaging::generation_finish::GenerationFinish;
    use paddler_messaging::generation_summary::GenerationSummary;

    use super::ChatCompletionHeader;
    use crate::completes_generation::CompletesGeneration as _;
    use crate::generated_output::GeneratedOutput;
    use crate::generated_output_part::GeneratedOutputPart;

    fn completion_of(
        generated_output_parts: Vec<GeneratedOutputPart>,
        prompt_tokens: u64,
        content_tokens: u64,
        reasoning_tokens: u64,
    ) -> String {
        let mut generated_output = GeneratedOutput::default();

        for generated_output_part in generated_output_parts {
            generated_output.append(generated_output_part);
        }

        to_string(
            &ChatCompletionHeader {
                created: 0,
                model: "test-model".to_owned(),
            }
            .complete(
                "test-request",
                generated_output,
                &GenerationSummary {
                    finish: GenerationFinish::EndOfGeneration,
                    usage: TokenUsage {
                        prompt_tokens,
                        content_tokens,
                        reasoning_tokens,
                        ..TokenUsage::default()
                    },
                },
            ),
        )
        .unwrap()
    }

    #[test]
    fn aggregates_content_into_one_message() {
        assert_eq!(
            completion_of(
                vec![
                    GeneratedOutputPart::Content("hel".to_owned()),
                    GeneratedOutputPart::Content("lo".to_owned()),
                ],
                4,
                2,
                0,
            ),
            r#"{"id":"test-request","object":"chat.completion","created":0,"model":"test-model","choices":[{"index":0,"message":{"role":"assistant","content":"hello","refusal":null,"annotations":[]},"logprobs":null,"finish_reason":"stop"}],"usage":{"prompt_tokens":4,"completion_tokens":2,"total_tokens":6,"prompt_tokens_details":{"cached_tokens":0,"audio_tokens":0},"completion_tokens_details":{"reasoning_tokens":0}},"service_tier":"default"}"#
        );
    }

    #[test]
    fn drops_reasoning_but_keeps_its_token_count() {
        assert_eq!(
            completion_of(
                vec![
                    GeneratedOutputPart::Reasoning("think".to_owned()),
                    GeneratedOutputPart::Content("answer".to_owned()),
                ],
                3,
                1,
                1,
            ),
            r#"{"id":"test-request","object":"chat.completion","created":0,"model":"test-model","choices":[{"index":0,"message":{"role":"assistant","content":"answer","refusal":null,"annotations":[]},"logprobs":null,"finish_reason":"stop"}],"usage":{"prompt_tokens":3,"completion_tokens":2,"total_tokens":5,"prompt_tokens_details":{"cached_tokens":0,"audio_tokens":0},"completion_tokens_details":{"reasoning_tokens":1}},"service_tier":"default"}"#
        );
    }

    #[test]
    fn reports_parsed_tool_calls_with_their_raw_arguments_and_the_tool_calls_finish_reason() {
        assert_eq!(
            completion_of(
                vec![GeneratedOutputPart::ToolCalls(vec![
                    ParsedToolCall::new(
                        "call_x".to_owned(),
                        "get_weather".to_owned(),
                        ToolCallArguments::ValidJson(json!({ "location": "Paris" })),
                    ),
                    ParsedToolCall::new(
                        "call_invalid".to_owned(),
                        "broken_tool".to_owned(),
                        ToolCallArguments::InvalidJson("{not valid json".to_owned()),
                    ),
                ])],
                4,
                0,
                0,
            ),
            r#"{"id":"test-request","object":"chat.completion","created":0,"model":"test-model","choices":[{"index":0,"message":{"role":"assistant","content":null,"refusal":null,"annotations":[],"tool_calls":[{"id":"call_x","type":"function","function":{"name":"get_weather","arguments":"{\"location\":\"Paris\"}"}},{"id":"call_invalid","type":"function","function":{"name":"broken_tool","arguments":"{not valid json"}}]},"logprobs":null,"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":4,"completion_tokens":0,"total_tokens":4,"prompt_tokens_details":{"cached_tokens":0,"audio_tokens":0},"completion_tokens_details":{"reasoning_tokens":0}},"service_tier":"default"}"#
        );
    }
}
