use std::num::NonZeroU32;
use std::time::SystemTime;

use nanoid::nanoid;
use serde::Deserialize;

use paddler_inference_parameters::sampling_parameters::SamplingParameters;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::validates::Validates;

use crate::default_max_tokens::DEFAULT_MAX_TOKENS;
use crate::openai_translation_error::OpenAITranslationError;
use crate::responses_delivery::ResponsesDelivery;
use crate::responses_input::ResponsesInput;
use crate::responses_input_item::ResponsesInputItem;
use crate::responses_reasoning::ResponsesReasoning;
use crate::responses_response_header::ResponsesResponseHeader;
use crate::responses_stream::ResponsesStream;
use crate::responses_text_param::ResponsesTextParam;
use crate::responses_tool::ResponsesTool;
use crate::timestamp_from::timestamp_from;
use crate::translated_responses_request::TranslatedResponsesRequest;

#[derive(Deserialize)]
pub struct ResponsesRequest {
    pub model: String,
    #[serde(default)]
    pub input: ResponsesInput,
    #[serde(default)]
    pub instructions: Option<String>,
    #[serde(default)]
    pub stream: Option<bool>,
    #[serde(default)]
    pub max_output_tokens: Option<NonZeroU32>,
    #[serde(default)]
    pub tools: Vec<ResponsesTool>,
    #[serde(default)]
    pub text: Option<ResponsesTextParam>,
    #[serde(default)]
    pub reasoning: Option<ResponsesReasoning>,
}

impl ResponsesRequest {
    pub fn translate(
        self,
        now: SystemTime,
        SamplingParameters {
            temperature, top_p, ..
        }: &SamplingParameters,
    ) -> Result<TranslatedResponsesRequest, OpenAITranslationError> {
        let Self {
            model,
            input,
            instructions,
            stream,
            max_output_tokens,
            tools,
            text,
            reasoning,
        } = self;

        let mut messages: Vec<ConversationMessage> = Vec::new();

        if let Some(instructions) = &instructions
            && !instructions.is_empty()
        {
            messages.push(ConversationMessage {
                content: ConversationMessageContent::Text(instructions.clone()),
                role: "system".to_owned(),
            });
        }

        match input {
            ResponsesInput::Text(text) => messages.push(ConversationMessage {
                content: ConversationMessageContent::Text(text),
                role: "user".to_owned(),
            }),
            ResponsesInput::Items(items) => {
                messages.extend(
                    items
                        .into_iter()
                        .map(ResponsesInputItem::into_conversation_message),
                );
            }
        }

        let validated_tools = tools
            .into_iter()
            .map(|ResponsesTool::Function(function_definition)| function_definition.into_tool())
            .map(Validates::validate)
            .collect::<Result<Vec<_>, _>>()?;

        let created_at = timestamp_from(now)?;
        let header = ResponsesResponseHeader {
            created_at,
            id: format!("resp_{}", nanoid!()),
            instructions,
            model,
            temperature: *temperature,
            top_p: *top_p,
        };

        Ok(TranslatedResponsesRequest {
            conversation_history_params: ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(messages),
                enable_thinking: reasoning
                    .as_ref()
                    .is_none_or(ResponsesReasoning::enables_thinking),
                grammar: text.and_then(ResponsesTextParam::into_grammar_constraint),
                max_tokens: max_output_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
                parse_tool_calls: !validated_tools.is_empty(),
                tools: validated_tools,
            },
            delivery: if stream.unwrap_or(false) {
                ResponsesDelivery::Streamed(ResponsesStream::new(header))
            } else {
                ResponsesDelivery::Buffered(header)
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;
    use std::time::UNIX_EPOCH;

    use serde_json::Value;
    use serde_json::error::Category;
    use serde_json::from_str;
    use serde_json::from_value;
    use serde_json::json;

    use paddler_inference_parameters::sampling_parameters::SamplingParameters;
    use paddler_messaging::conversation_message_content::ConversationMessageContent;
    use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;
    use paddler_messaging::grammar_constraint::GrammarConstraint;
    use paddler_messaging::image_url::ImageUrl;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
    use paddler_messaging::request_params_validation_error::RequestParamsValidationError;

    use super::ResponsesRequest;
    use crate::openai_translation_error::OpenAITranslationError;
    use crate::translated_responses_request::TranslatedResponsesRequest;

    fn translated_from(value: Value) -> TranslatedResponsesRequest {
        let params: ResponsesRequest = from_value(value).unwrap();

        params
            .translate(UNIX_EPOCH, &SamplingParameters::default())
            .unwrap()
    }

    #[test]
    fn string_input_becomes_a_single_user_message() {
        let translated = translated_from(json!({ "model": "test", "input": "Say hello" }));

        let messages = &translated
            .conversation_history_params
            .conversation_history
            .messages;

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[0].content.text_content(), "Say hello");
    }

    #[test]
    fn instructions_are_prepended_as_a_system_message() {
        let translated = translated_from(json!({
            "model": "test",
            "instructions": "be terse",
            "input": "hi"
        }));

        let messages = &translated
            .conversation_history_params
            .conversation_history
            .messages;

        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[0].content.text_content(), "be terse");
        assert_eq!(messages[1].role, "user");
    }

    #[test]
    fn function_call_output_item_becomes_a_tool_message() {
        let translated = translated_from(json!({
            "model": "test",
            "input": [
                { "type": "function_call_output", "call_id": "call_1", "output": "sunny" }
            ]
        }));

        let messages = &translated
            .conversation_history_params
            .conversation_history
            .messages;

        assert_eq!(messages[0].role, "tool");
        assert_eq!(messages[0].content.text_content(), "sunny");
    }

    #[test]
    fn function_call_item_becomes_an_assistant_message_carrying_the_call() {
        let translated = translated_from(json!({
            "model": "test",
            "input": [
                { "type": "function_call", "call_id": "call_1", "name": "get_weather", "arguments": "{}" }
            ]
        }));

        let messages = &translated
            .conversation_history_params
            .conversation_history
            .messages;

        assert_eq!(messages[0].role, "assistant");
        assert_eq!(
            from_str::<Value>(&messages[0].content.text_content()).unwrap(),
            json!({ "call_id": "call_1", "name": "get_weather", "arguments": "{}" })
        );
    }

    #[test]
    fn message_content_parts_become_conversation_parts() {
        let translated = translated_from(json!({
            "model": "test",
            "input": [
                {
                    "type": "message",
                    "role": "user",
                    "content": [
                        { "type": "input_text", "text": "What is this?" },
                        { "type": "input_image", "image_url": "https://example.test/cat.png" }
                    ]
                }
            ]
        }));

        assert_eq!(
            translated
                .conversation_history_params
                .conversation_history
                .messages[0]
                .content,
            ConversationMessageContent::Parts(vec![
                ConversationMessageContentPart::Text {
                    text: "What is this?".to_owned(),
                },
                ConversationMessageContentPart::ImageUrl {
                    image_url: ImageUrl {
                        url: "https://example.test/cat.png".to_owned(),
                    },
                },
            ])
        );
    }

    #[test]
    fn text_format_text_leaves_the_output_unconstrained() {
        let translated = translated_from(json!({
            "model": "test",
            "input": "hi",
            "text": { "format": { "type": "text" } }
        }));

        assert_eq!(translated.conversation_history_params.grammar, None);
    }

    #[test]
    fn developer_role_is_normalized_to_system() {
        let translated = translated_from(json!({
            "model": "test",
            "input": [
                { "type": "message", "role": "developer", "content": "rules" }
            ]
        }));

        assert_eq!(
            translated
                .conversation_history_params
                .conversation_history
                .messages[0]
                .role,
            "system"
        );
    }

    #[test]
    fn flat_function_tool_maps_to_an_internal_tool_with_default_description() {
        let translated = translated_from(json!({
            "model": "test",
            "input": "hi",
            "tools": [
                { "type": "function", "name": "get_weather", "parameters": { "type": "object" } }
            ]
        }));

        assert!(translated.conversation_history_params.parse_tool_calls);

        let Tool::Function(function_call) = &translated.conversation_history_params.tools[0];

        assert_eq!(function_call.function.name, "get_weather");
        assert_eq!(function_call.function.description, "");
    }

    #[test]
    fn text_format_json_schema_becomes_a_grammar_constraint() {
        let translated = translated_from(json!({
            "model": "test",
            "input": "hi",
            "text": { "format": { "type": "json_schema", "name": "out", "schema": { "type": "object" } } }
        }));

        assert!(matches!(
            &translated.conversation_history_params.grammar,
            Some(GrammarConstraint::JsonSchema { schema }) if schema == r#"{"type":"object"}"#
        ));
    }

    #[test]
    fn a_request_without_an_output_limit_generates_until_its_context_is_full() {
        let translated = translated_from(json!({ "model": "test", "input": "hi" }));

        assert_eq!(
            translated.conversation_history_params.max_tokens,
            NonZeroU32::MAX
        );
    }

    #[test]
    fn reasoning_effort_none_disables_thinking() {
        let translated = translated_from(json!({
            "model": "test",
            "input": "hi",
            "reasoning": { "effort": "none" }
        }));

        assert!(!translated.conversation_history_params.enable_thinking);
    }

    #[test]
    fn rejects_input_it_cannot_represent() {
        let unrepresentable_requests = [
            json!({ "model": "test", "input": "hi", "tools": [ { "type": "web_search" } ] }),
            json!({ "model": "test", "input": [ { "type": "reasoning", "summary": [] } ] }),
            json!({ "model": "test", "input": [ { "type": "message", "role": "user", "content": [ { "type": "input_file", "file_id": "file_1" } ] } ] }),
            json!({ "model": "test", "input": [ { "type": "message", "role": "user", "content": [ { "type": "input_image", "file_id": "file_1" } ] } ] }),
            json!({ "model": "test", "input": [ { "type": "function_call_output", "call_id": "call_1", "output": [ { "type": "input_image", "image_url": "https://example.test/cat.png" } ] } ] }),
            json!({ "model": "test", "input": "hi", "text": { "format": { "type": "json_object" } } }),
        ];

        for unrepresentable_request in unrepresentable_requests {
            let rejection = from_value::<ResponsesRequest>(unrepresentable_request)
                .err()
                .map(|deserialization_error| deserialization_error.classify());

            assert_eq!(rejection, Some(Category::Data));
        }
    }

    #[test]
    fn unsupported_and_stateful_fields_are_ignored() {
        let translated = translated_from(json!({
            "model": "test",
            "input": "hi",
            "store": true,
            "previous_response_id": "resp_prev",
            "conversation": "conv_1",
            "temperature": 0.5,
            "tool_choice": "required"
        }));

        assert_eq!(
            translated
                .conversation_history_params
                .conversation_history
                .messages
                .len(),
            1
        );
    }

    #[test]
    fn instructions_alone_become_the_whole_conversation() {
        let translated = translated_from(json!({ "model": "test", "instructions": "be terse" }));

        let messages = &translated
            .conversation_history_params
            .conversation_history
            .messages;

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[0].content.text_content(), "be terse");
    }

    #[test]
    fn message_item_without_a_type_becomes_a_conversation_message() {
        let translated = translated_from(json!({
            "model": "test",
            "input": [ { "role": "user", "content": "hi" } ]
        }));

        let messages = &translated
            .conversation_history_params
            .conversation_history
            .messages;

        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[0].content.text_content(), "hi");
    }

    #[test]
    fn rejects_a_function_tool_that_requires_an_undeclared_property() {
        let params: ResponsesRequest = from_value(json!({
            "model": "test",
            "input": "hi",
            "tools": [
                {
                    "type": "function",
                    "name": "broken",
                    "parameters": {
                        "type": "object",
                        "properties": { "present": { "type": "string" } },
                        "required": ["absent"]
                    }
                }
            ]
        }))
        .unwrap();

        assert!(matches!(
            params.translate(UNIX_EPOCH, &SamplingParameters::default()),
            Err(OpenAITranslationError::ToolRejected(
                RequestParamsValidationError::RequiredFieldNotInProperties { field }
            )) if field == "absent"
        ));
    }
}
