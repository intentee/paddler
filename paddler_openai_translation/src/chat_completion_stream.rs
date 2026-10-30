use paddler_messaging::generated_tool_call::GeneratedToolCall;
use paddler_messaging::generation_summary::GenerationSummary;

use crate::chat_completion_chunk::ChatCompletionChunk;
use crate::chat_completion_chunk_choice::ChatCompletionChunkChoice;
use crate::chat_completion_chunk_payload::ChatCompletionChunkPayload;
use crate::chat_completion_finish_reason::chat_completion_finish_reason;
use crate::chat_completion_header::ChatCompletionHeader;
use crate::chat_completion_stream_params::ChatCompletionStreamParams;
use crate::generated_output_part::GeneratedOutputPart;
use crate::generation_failure::GenerationFailure;
use crate::generation_outcome::GenerationOutcome;
use crate::generation_progress::GenerationProgress;

pub struct ChatCompletionStream {
    header: ChatCompletionHeader,
    include_usage: bool,
    saw_tool_call: bool,
    system_fingerprint: String,
}

impl ChatCompletionStream {
    #[must_use]
    pub fn new(
        ChatCompletionStreamParams {
            header,
            include_usage,
            system_fingerprint,
        }: ChatCompletionStreamParams,
    ) -> Self {
        Self {
            header,
            include_usage,
            saw_tool_call: false,
            system_fingerprint,
        }
    }

    pub fn advance(
        &mut self,
        request_id: &str,
        generation_progress: GenerationProgress,
    ) -> Result<Vec<ChatCompletionChunk>, GenerationFailure> {
        match generation_progress {
            GenerationProgress::Produced(GeneratedOutputPart::Content(text)) => {
                Ok(vec![self.chunk(
                    request_id,
                    ChatCompletionChunkPayload::Choice(ChatCompletionChunkChoice::Content(text)),
                )])
            }
            GenerationProgress::Produced(GeneratedOutputPart::Reasoning(_))
            | GenerationProgress::ToolCallTokenProduced => Ok(Vec::new()),
            GenerationProgress::Produced(GeneratedOutputPart::ToolCalls(parsed_calls)) => {
                Ok(self.tool_calls(request_id, parsed_calls))
            }
            GenerationProgress::Failed(generation_failure) => Err(generation_failure),
        }
    }

    pub fn conclude(
        &self,
        request_id: &str,
        generation_outcome: GenerationOutcome,
    ) -> Result<Vec<ChatCompletionChunk>, GenerationFailure> {
        match generation_outcome {
            GenerationOutcome::Failed(generation_failure) => Err(generation_failure),
            GenerationOutcome::Finished(generation_summary) => {
                Ok(self.finish(request_id, &generation_summary))
            }
        }
    }

    fn chunk(&self, request_id: &str, payload: ChatCompletionChunkPayload) -> ChatCompletionChunk {
        ChatCompletionChunk {
            created: self.header.created,
            id: request_id.to_owned(),
            model: self.header.model.clone(),
            payload,
            system_fingerprint: self.system_fingerprint.clone(),
        }
    }

    fn finish(
        &self,
        request_id: &str,
        GenerationSummary { finish, usage }: &GenerationSummary,
    ) -> Vec<ChatCompletionChunk> {
        let finish_chunk = self.chunk(
            request_id,
            ChatCompletionChunkPayload::Choice(ChatCompletionChunkChoice::Finish(
                chat_completion_finish_reason(*finish, self.saw_tool_call),
            )),
        );

        if self.include_usage {
            vec![
                finish_chunk,
                self.chunk(request_id, ChatCompletionChunkPayload::Usage(*usage)),
            ]
        } else {
            vec![finish_chunk]
        }
    }

    fn tool_calls(
        &mut self,
        request_id: &str,
        parsed_calls: Vec<GeneratedToolCall>,
    ) -> Vec<ChatCompletionChunk> {
        if parsed_calls.is_empty() {
            return Vec::new();
        }

        self.saw_tool_call = true;

        vec![self.chunk(
            request_id,
            ChatCompletionChunkPayload::Choice(ChatCompletionChunkChoice::ToolCalls(parsed_calls)),
        )]
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings_types::TokenUsage;
    use llama_cpp_bindings_types::ToolCallArguments;
    use serde_json::json;
    use serde_json::to_string;

    use paddler_messaging::generated_tool_call::GeneratedToolCall;
    use paddler_messaging::generation_finish::GenerationFinish;
    use paddler_messaging::generation_summary::GenerationSummary;

    use super::ChatCompletionStream;
    use crate::chat_completion_chunk::ChatCompletionChunk;
    use crate::chat_completion_header::ChatCompletionHeader;
    use crate::chat_completion_stream_params::ChatCompletionStreamParams;
    use crate::generated_output_part::GeneratedOutputPart;
    use crate::generation_failure::GenerationFailure;
    use crate::generation_failure_cause::GenerationFailureCause;
    use crate::generation_outcome::GenerationOutcome;
    use crate::generation_progress::GenerationProgress;

    fn chat_completion_stream(include_usage: bool) -> ChatCompletionStream {
        ChatCompletionStream::new(ChatCompletionStreamParams {
            header: ChatCompletionHeader {
                created: 0,
                model: "test-model".to_owned(),
            },
            include_usage,
            system_fingerprint: "test-fingerprint".to_owned(),
        })
    }

    fn finished(finish: GenerationFinish) -> GenerationOutcome {
        GenerationOutcome::Finished(GenerationSummary {
            finish,
            usage: TokenUsage {
                prompt_tokens: 7,
                content_tokens: 4,
                reasoning_tokens: 1,
                ..TokenUsage::default()
            },
        })
    }

    fn serialized_chunks(
        chat_completion_chunks: Result<Vec<ChatCompletionChunk>, GenerationFailure>,
    ) -> Vec<String> {
        chat_completion_chunks
            .unwrap()
            .iter()
            .map(|chat_completion_chunk| to_string(chat_completion_chunk).unwrap())
            .collect()
    }

    fn tool_call(
        index: usize,
        id: &str,
        name: &str,
        arguments: ToolCallArguments,
    ) -> GeneratedToolCall {
        GeneratedToolCall {
            arguments,
            id: id.to_owned(),
            index,
            name: name.to_owned(),
        }
    }

    #[test]
    fn emits_a_content_delta_for_produced_content() {
        assert_eq!(
            serialized_chunks(chat_completion_stream(false).advance(
                "test-request",
                GenerationProgress::Produced(GeneratedOutputPart::Content("hello".to_owned())),
            )),
            vec![
                r#"{"id":"test-request","object":"chat.completion.chunk","created":0,"model":"test-model","system_fingerprint":"test-fingerprint","choices":[{"index":0,"delta":{"role":"assistant","content":"hello"},"logprobs":null,"finish_reason":null}]}"#,
            ]
        );
    }

    #[test]
    fn drops_reasoning_and_tool_call_tokens() {
        let mut chat_completion_stream = chat_completion_stream(false);

        for generation_progress in [
            GenerationProgress::Produced(GeneratedOutputPart::Reasoning("thought".to_owned())),
            GenerationProgress::ToolCallTokenProduced,
        ] {
            assert_eq!(
                serialized_chunks(
                    chat_completion_stream.advance("test-request", generation_progress)
                ),
                Vec::<String>::new()
            );
        }
    }

    #[test]
    fn emits_parsed_tool_calls_with_their_raw_arguments() {
        assert_eq!(
            serialized_chunks(chat_completion_stream(false).advance(
                "test-request",
                GenerationProgress::Produced(GeneratedOutputPart::ToolCalls(vec![
                    tool_call(
                        0,
                        "call_x",
                        "get_weather",
                        ToolCallArguments::ValidJson(json!({ "location": "Paris" })),
                    ),
                    tool_call(
                        1,
                        "call_invalid",
                        "broken_tool",
                        ToolCallArguments::InvalidJson("{not valid json".to_owned()),
                    ),
                ])),
            )),
            vec![
                r#"{"id":"test-request","object":"chat.completion.chunk","created":0,"model":"test-model","system_fingerprint":"test-fingerprint","choices":[{"index":0,"delta":{"role":"assistant","tool_calls":[{"index":0,"id":"call_x","type":"function","function":{"name":"get_weather","arguments":"{\"location\":\"Paris\"}"}},{"index":1,"id":"call_invalid","type":"function","function":{"name":"broken_tool","arguments":"{not valid json"}}]},"logprobs":null,"finish_reason":null}]}"#,
            ]
        );
    }

    #[test]
    fn indexes_tool_calls_of_separate_sections_by_their_position_in_the_generation() {
        let mut chat_completion_stream = chat_completion_stream(false);
        let tool_call_chunks: Vec<String> = [
            tool_call(
                0,
                "call_0",
                "get_weather",
                ToolCallArguments::ValidJson(json!({ "location": "Paris" })),
            ),
            tool_call(
                1,
                "call_1",
                "get_time",
                ToolCallArguments::ValidJson(json!({ "timezone": "CET" })),
            ),
        ]
        .into_iter()
        .flat_map(|generated_tool_call| {
            serialized_chunks(chat_completion_stream.advance(
                "test-request",
                GenerationProgress::Produced(GeneratedOutputPart::ToolCalls(vec![
                    generated_tool_call,
                ])),
            ))
        })
        .collect();

        assert_eq!(
            tool_call_chunks,
            vec![
                r#"{"id":"test-request","object":"chat.completion.chunk","created":0,"model":"test-model","system_fingerprint":"test-fingerprint","choices":[{"index":0,"delta":{"role":"assistant","tool_calls":[{"index":0,"id":"call_0","type":"function","function":{"name":"get_weather","arguments":"{\"location\":\"Paris\"}"}}]},"logprobs":null,"finish_reason":null}]}"#,
                r#"{"id":"test-request","object":"chat.completion.chunk","created":0,"model":"test-model","system_fingerprint":"test-fingerprint","choices":[{"index":0,"delta":{"role":"assistant","tool_calls":[{"index":1,"id":"call_1","type":"function","function":{"name":"get_time","arguments":"{\"timezone\":\"CET\"}"}}]},"logprobs":null,"finish_reason":null}]}"#,
            ]
        );
    }

    #[test]
    fn emits_nothing_for_empty_parsed_tool_calls() {
        let mut chat_completion_stream = chat_completion_stream(false);

        assert_eq!(
            serialized_chunks(chat_completion_stream.advance(
                "test-request",
                GenerationProgress::Produced(GeneratedOutputPart::ToolCalls(Vec::new())),
            )),
            Vec::<String>::new()
        );
        assert_eq!(
            serialized_chunks(
                chat_completion_stream
                    .conclude("test-request", finished(GenerationFinish::EndOfGeneration))
            ),
            vec![
                r#"{"id":"test-request","object":"chat.completion.chunk","created":0,"model":"test-model","system_fingerprint":"test-fingerprint","choices":[{"index":0,"delta":{},"logprobs":null,"finish_reason":"stop"}]}"#,
            ]
        );
    }

    #[test]
    fn finishes_with_tool_calls_after_a_tool_call_and_length_at_a_length_limit() {
        let mut after_tool_call = chat_completion_stream(false);

        serialized_chunks(after_tool_call.advance(
            "test-request",
            GenerationProgress::Produced(GeneratedOutputPart::ToolCalls(vec![tool_call(
                0,
                "call_x",
                "get_weather",
                ToolCallArguments::ValidJson(json!({ "location": "Paris" })),
            )])),
        ));

        assert_eq!(
            serialized_chunks(
                after_tool_call
                    .conclude("test-request", finished(GenerationFinish::EndOfGeneration))
            ),
            vec![
                r#"{"id":"test-request","object":"chat.completion.chunk","created":0,"model":"test-model","system_fingerprint":"test-fingerprint","choices":[{"index":0,"delta":{},"logprobs":null,"finish_reason":"tool_calls"}]}"#,
            ]
        );
        assert_eq!(
            serialized_chunks(
                chat_completion_stream(false)
                    .conclude("test-request", finished(GenerationFinish::MaxTokens))
            ),
            vec![
                r#"{"id":"test-request","object":"chat.completion.chunk","created":0,"model":"test-model","system_fingerprint":"test-fingerprint","choices":[{"index":0,"delta":{},"logprobs":null,"finish_reason":"length"}]}"#,
            ]
        );
    }

    #[test]
    fn appends_a_usage_chunk_when_the_client_asks_for_usage() {
        assert_eq!(
            serialized_chunks(
                chat_completion_stream(true)
                    .conclude("test-request", finished(GenerationFinish::EndOfGeneration))
            ),
            vec![
                r#"{"id":"test-request","object":"chat.completion.chunk","created":0,"model":"test-model","system_fingerprint":"test-fingerprint","choices":[{"index":0,"delta":{},"logprobs":null,"finish_reason":"stop"}]}"#,
                r#"{"id":"test-request","object":"chat.completion.chunk","created":0,"model":"test-model","system_fingerprint":"test-fingerprint","choices":[],"usage":{"prompt_tokens":7,"completion_tokens":5,"total_tokens":12,"prompt_tokens_details":{"cached_tokens":0,"audio_tokens":0},"completion_tokens_details":{"reasoning_tokens":1}}}"#,
            ]
        );
    }

    #[test]
    fn passes_generation_failures_through() {
        let generation_failure = || GenerationFailure {
            cause: GenerationFailureCause::AgentFailed,
            message: "sampler blew up".to_owned(),
        };
        let mut chat_completion_stream = chat_completion_stream(false);

        assert_eq!(
            chat_completion_stream
                .advance(
                    "test-request",
                    GenerationProgress::Failed(generation_failure())
                )
                .err(),
            Some(generation_failure())
        );
        assert_eq!(
            chat_completion_stream
                .conclude(
                    "test-request",
                    GenerationOutcome::Failed(generation_failure())
                )
                .err(),
            Some(generation_failure())
        );
    }
}
