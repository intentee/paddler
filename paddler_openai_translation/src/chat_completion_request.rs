use std::num::NonZeroU32;
use std::time::SystemTime;

use nanoid::nanoid;
use serde::Deserialize;

use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::validates::Validates;

use crate::chat_completion_delivery::ChatCompletionDelivery;
use crate::chat_completion_header::ChatCompletionHeader;
use crate::chat_completion_message::ChatCompletionMessage;
use crate::chat_completion_stream::ChatCompletionStream;
use crate::chat_completion_stream_options::ChatCompletionStreamOptions;
use crate::chat_completion_stream_params::ChatCompletionStreamParams;
use crate::chat_completion_tool::ChatCompletionTool;
use crate::default_max_tokens::DEFAULT_MAX_TOKENS;
use crate::openai_translation_error::OpenAITranslationError;
use crate::reasoning_effort::ReasoningEffort;
use crate::timestamp_from::timestamp_from;
use crate::translated_chat_completion_request::TranslatedChatCompletionRequest;

#[derive(Deserialize)]
pub struct ChatCompletionRequest {
    pub max_completion_tokens: Option<NonZeroU32>,
    pub max_tokens: Option<NonZeroU32>,
    pub messages: Vec<ChatCompletionMessage>,
    pub model: String,
    pub reasoning_effort: Option<ReasoningEffort>,
    pub stream: Option<bool>,
    pub stream_options: Option<ChatCompletionStreamOptions>,
    #[serde(default)]
    pub tools: Vec<ChatCompletionTool>,
}

impl ChatCompletionRequest {
    #[must_use]
    pub fn enables_thinking(&self) -> bool {
        self.reasoning_effort
            .is_none_or(ReasoningEffort::enables_thinking)
    }

    #[must_use]
    pub fn requested_max_tokens(&self) -> Option<NonZeroU32> {
        self.max_completion_tokens.or(self.max_tokens)
    }

    pub fn translate(
        self,
        now: SystemTime,
    ) -> Result<TranslatedChatCompletionRequest, OpenAITranslationError> {
        let enable_thinking = self.enables_thinking();
        let max_tokens = self.requested_max_tokens().unwrap_or(DEFAULT_MAX_TOKENS);
        let Self {
            messages,
            model,
            stream,
            stream_options,
            tools,
            ..
        } = self;
        let validated_tools = tools
            .into_iter()
            .map(ChatCompletionTool::into_tool)
            .map(Validates::validate)
            .collect::<Result<Vec<_>, _>>()?;
        let mut conversation_messages = Vec::with_capacity(messages.len());

        for message in messages {
            message.append_to(&mut conversation_messages)?;
        }

        let header = ChatCompletionHeader {
            created: timestamp_from(now)?,
            model,
        };

        Ok(TranslatedChatCompletionRequest {
            conversation_history_params: ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(conversation_messages),
                enable_thinking,
                grammar: None,
                max_tokens,
                parse_tool_calls: !validated_tools.is_empty(),
                tools: validated_tools,
            },
            delivery: if stream.unwrap_or(false) {
                ChatCompletionDelivery::Streamed(ChatCompletionStream::new(
                    ChatCompletionStreamParams {
                        header,
                        include_usage: stream_options.is_some_and(
                            |ChatCompletionStreamOptions { include_usage }| include_usage,
                        ),
                        system_fingerprint: nanoid!(),
                    },
                ))
            } else {
                ChatCompletionDelivery::Buffered(header)
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;
    use std::time::UNIX_EPOCH;

    use serde_json::Value;
    use serde_json::from_value;
    use serde_json::json;

    use paddler_messaging::conversation_message::ConversationMessage;
    use paddler_messaging::conversation_message_content::ConversationMessageContent;
    use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;
    use paddler_messaging::image_url::ImageUrl;

    use super::ChatCompletionRequest;
    use crate::chat_completion_message_role::ChatCompletionMessageRole;
    use crate::openai_translation_error::OpenAITranslationError;

    fn conversation_messages_of(request: ChatCompletionRequest) -> Vec<ConversationMessage> {
        let mut conversation_messages = Vec::new();

        for message in request.messages {
            message.append_to(&mut conversation_messages).unwrap();
        }

        conversation_messages
    }

    fn request_with_token_limits(token_limits: &Value) -> ChatCompletionRequest {
        let mut input = json!({
            "model": "test-model",
            "messages": [{"role": "user", "content": "hello"}]
        });

        input
            .as_object_mut()
            .unwrap()
            .extend(token_limits.as_object().unwrap().clone());

        from_value(input).unwrap()
    }

    #[test]
    fn requested_max_tokens_honors_max_tokens() {
        let params = request_with_token_limits(&json!({"max_tokens": 7}));

        assert_eq!(params.requested_max_tokens(), NonZeroU32::new(7));
    }

    #[test]
    fn requested_max_tokens_prefers_max_completion_tokens() {
        let params =
            request_with_token_limits(&json!({"max_tokens": 7, "max_completion_tokens": 3}));

        assert_eq!(params.requested_max_tokens(), NonZeroU32::new(3));
    }

    #[test]
    fn requested_max_tokens_is_absent_without_limits() {
        let params = request_with_token_limits(&json!({}));

        assert_eq!(params.requested_max_tokens(), None);
    }

    #[test]
    fn deserialize_text_only_request() {
        let input = json!({
            "model": "test-model",
            "messages": [
                {"role": "user", "content": "hello"}
            ]
        });

        let params: ChatCompletionRequest = from_value(input).unwrap();

        assert_eq!(params.model, "test-model");
        assert_eq!(
            conversation_messages_of(params),
            vec![ConversationMessage::Message {
                content: ConversationMessageContent::Text("hello".to_owned()),
                role: "user".to_owned(),
            }]
        );
    }

    #[test]
    fn a_request_with_a_message_it_cannot_translate_is_refused() {
        let params: ChatCompletionRequest = from_value(json!({
            "model": "test-model",
            "messages": [{"role": "user", "content": null}]
        }))
        .unwrap();

        assert_eq!(
            params
                .translate(UNIX_EPOCH)
                .err()
                .map(|translation_error| translation_error.to_string()),
            Some(
                OpenAITranslationError::MessageWithoutContent {
                    role: ChatCompletionMessageRole::User
                }
                .to_string()
            )
        );
    }

    #[test]
    fn deserialize_request_with_stream_options_include_usage_true() {
        let input = json!({
            "model": "test-model",
            "messages": [{"role": "user", "content": "hi"}],
            "stream": true,
            "stream_options": {"include_usage": true}
        });

        let params: ChatCompletionRequest = from_value(input).unwrap();

        let stream_options = params.stream_options.unwrap();

        assert!(stream_options.include_usage);
    }

    #[test]
    fn deserialize_request_without_stream_options_defaults_to_none() {
        let input = json!({
            "model": "test-model",
            "messages": [{"role": "user", "content": "hi"}],
            "stream": true
        });

        let params: ChatCompletionRequest = from_value(input).unwrap();

        assert!(params.stream_options.is_none());
    }

    #[test]
    fn deserialize_multimodal_request_with_image() {
        let input = json!({
            "model": "vision-model",
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {"type": "text", "text": "describe this image"},
                        {"type": "image_url", "image_url": {"url": "data:image/jpeg;base64,/9j/4AAQ"}}
                    ]
                }
            ]
        });

        assert_eq!(
            conversation_messages_of(from_value::<ChatCompletionRequest>(input).unwrap()),
            vec![ConversationMessage::Message {
                content: ConversationMessageContent::Parts(vec![
                    ConversationMessageContentPart::Text {
                        text: "describe this image".to_owned(),
                    },
                    ConversationMessageContentPart::ImageUrl {
                        image_url: ImageUrl {
                            url: "data:image/jpeg;base64,/9j/4AAQ".to_owned(),
                        },
                    },
                ]),
                role: "user".to_owned(),
            }]
        );
    }

    #[test]
    fn deserialize_multi_turn_conversation() {
        let input = json!({
            "model": "test-model",
            "messages": [
                {"role": "system", "content": "You are a helpful assistant."},
                {"role": "user", "content": "What is 2+2?"},
                {"role": "assistant", "content": "4"},
                {"role": "user", "content": "And 3+3?"}
            ]
        });

        let params: ChatCompletionRequest = from_value(input).unwrap();

        assert_eq!(params.messages.len(), 4);
    }

    #[test]
    fn deserialize_request_with_opencode_style_tools() {
        let input = json!({
            "model": "test-model",
            "messages": [{"role": "user", "content": "hi"}],
            "tools": [
                {
                    "type": "function",
                    "function": {
                        "name": "glob",
                        "description": "Fast file pattern matching tool",
                        "parameters": {
                            "$schema": "https://json-schema.org/draft/2020-12/schema",
                            "type": "object",
                            "properties": {
                                "pattern": {"type": "string", "description": "The glob pattern"},
                                "path": {"type": "string", "description": "The directory to search in"}
                            },
                            "required": ["pattern"]
                        }
                    }
                },
                {
                    "type": "function",
                    "function": {
                        "name": "read",
                        "description": "Read a file from the local filesystem",
                        "parameters": {
                            "$schema": "https://json-schema.org/draft/2020-12/schema",
                            "type": "object",
                            "properties": {
                                "filePath": {"type": "string", "description": "The absolute path"},
                                "offset": {
                                    "minimum": 0,
                                    "type": "integer",
                                    "description": "The line number to start reading from"
                                }
                            },
                            "required": ["filePath"]
                        }
                    }
                }
            ]
        });

        let params: ChatCompletionRequest = from_value(input).unwrap();

        assert_eq!(params.tools.len(), 2);
    }
}
