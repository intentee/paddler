use std::num::NonZeroU32;

use anyhow::Result;
use serde::Deserialize;

use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::validates::Validates;

use crate::compatibility::openai_service::openai_default_max_tokens::OPENAI_DEFAULT_MAX_TOKENS;
use crate::compatibility::openai_service::openai_responses_input::OpenAIResponsesInput;
use crate::compatibility::openai_service::openai_responses_input_item::OpenAIResponsesInputItem;
use crate::compatibility::openai_service::openai_responses_reasoning::OpenAIResponsesReasoning;
use crate::compatibility::openai_service::openai_responses_text_param::OpenAIResponsesTextParam;
use crate::compatibility::openai_service::openai_responses_tool::OpenAIResponsesTool;
use crate::compatibility::openai_service::responses_prepared_request::ResponsesPreparedRequest;

#[derive(Deserialize)]
pub struct OpenAIResponsesRequestParams {
    pub model: String,
    #[serde(default)]
    pub input: OpenAIResponsesInput,
    #[serde(default)]
    pub instructions: Option<String>,
    #[serde(default)]
    pub stream: Option<bool>,
    #[serde(default)]
    pub max_output_tokens: Option<NonZeroU32>,
    #[serde(default)]
    pub tools: Vec<OpenAIResponsesTool>,
    #[serde(default)]
    pub text: Option<OpenAIResponsesTextParam>,
    #[serde(default)]
    pub reasoning: Option<OpenAIResponsesReasoning>,
}

impl OpenAIResponsesRequestParams {
    pub fn into_prepared(self) -> Result<ResponsesPreparedRequest> {
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
            OpenAIResponsesInput::Text(text) => messages.push(ConversationMessage {
                content: ConversationMessageContent::Text(text),
                role: "user".to_owned(),
            }),
            OpenAIResponsesInput::Items(items) => {
                messages.extend(
                    items
                        .into_iter()
                        .map(OpenAIResponsesInputItem::into_conversation_message),
                );
            }
        }

        let validated_tools = tools
            .into_iter()
            .map(|OpenAIResponsesTool::Function(function_definition)| {
                function_definition.into_tool()
            })
            .map(Validates::validate)
            .collect::<Result<Vec<_>, _>>()?;

        let parse_tool_calls = !validated_tools.is_empty();

        Ok(ResponsesPreparedRequest {
            paddler_params: ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(messages),
                enable_thinking: reasoning
                    .as_ref()
                    .is_none_or(OpenAIResponsesReasoning::enables_thinking),
                grammar: text.and_then(OpenAIResponsesTextParam::into_grammar_constraint),
                max_tokens: max_output_tokens.unwrap_or(OPENAI_DEFAULT_MAX_TOKENS),
                parse_tool_calls,
                tools: validated_tools,
            },
            stream: stream.unwrap_or(false),
            model,
            instructions,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use serde_json::Value;
    use serde_json::error::Category;
    use serde_json::from_str;
    use serde_json::from_value;
    use serde_json::json;

    use paddler_messaging::conversation_message_content::ConversationMessageContent;
    use paddler_messaging::conversation_message_content_part::ConversationMessageContentPart;
    use paddler_messaging::grammar_constraint::GrammarConstraint;
    use paddler_messaging::image_url::ImageUrl;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
    use paddler_messaging::request_params_validation_error::RequestParamsValidationError;

    use super::OpenAIResponsesRequestParams;
    use crate::compatibility::openai_service::responses_prepared_request::ResponsesPreparedRequest;

    fn prepared_from(value: Value) -> ResponsesPreparedRequest {
        let params: OpenAIResponsesRequestParams = from_value(value).unwrap();

        params.into_prepared().unwrap()
    }

    #[test]
    fn string_input_becomes_a_single_user_message() {
        let prepared = prepared_from(json!({ "model": "test", "input": "Say hello" }));

        let messages = &prepared.paddler_params.conversation_history.messages;

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[0].content.text_content(), "Say hello");
    }

    #[test]
    fn instructions_are_prepended_as_a_system_message() {
        let prepared = prepared_from(json!({
            "model": "test",
            "instructions": "be terse",
            "input": "hi"
        }));

        let messages = &prepared.paddler_params.conversation_history.messages;

        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[0].content.text_content(), "be terse");
        assert_eq!(messages[1].role, "user");
    }

    #[test]
    fn function_call_output_item_becomes_a_tool_message() {
        let prepared = prepared_from(json!({
            "model": "test",
            "input": [
                { "type": "function_call_output", "call_id": "call_1", "output": "sunny" }
            ]
        }));

        let messages = &prepared.paddler_params.conversation_history.messages;

        assert_eq!(messages[0].role, "tool");
        assert_eq!(messages[0].content.text_content(), "sunny");
    }

    #[test]
    fn function_call_item_becomes_an_assistant_message_carrying_the_call() {
        let prepared = prepared_from(json!({
            "model": "test",
            "input": [
                { "type": "function_call", "call_id": "call_1", "name": "get_weather", "arguments": "{}" }
            ]
        }));

        let messages = &prepared.paddler_params.conversation_history.messages;

        assert_eq!(messages[0].role, "assistant");
        assert_eq!(
            from_str::<Value>(&messages[0].content.text_content()).unwrap(),
            json!({ "call_id": "call_1", "name": "get_weather", "arguments": "{}" })
        );
    }

    #[test]
    fn message_content_parts_become_conversation_parts() {
        let prepared = prepared_from(json!({
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
            prepared.paddler_params.conversation_history.messages[0].content,
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
        let prepared = prepared_from(json!({
            "model": "test",
            "input": "hi",
            "text": { "format": { "type": "text" } }
        }));

        assert_eq!(prepared.paddler_params.grammar, None);
    }

    #[test]
    fn developer_role_is_normalized_to_system() {
        let prepared = prepared_from(json!({
            "model": "test",
            "input": [
                { "type": "message", "role": "developer", "content": "rules" }
            ]
        }));

        assert_eq!(
            prepared.paddler_params.conversation_history.messages[0].role,
            "system"
        );
    }

    #[test]
    fn flat_function_tool_maps_to_an_internal_tool_with_default_description() {
        let prepared = prepared_from(json!({
            "model": "test",
            "input": "hi",
            "tools": [
                { "type": "function", "name": "get_weather", "parameters": { "type": "object" } }
            ]
        }));

        assert!(prepared.paddler_params.parse_tool_calls);

        let Tool::Function(function_call) = &prepared.paddler_params.tools[0];

        assert_eq!(function_call.function.name, "get_weather");
        assert_eq!(function_call.function.description, "");
    }

    #[test]
    fn text_format_json_schema_becomes_a_grammar_constraint() {
        let prepared = prepared_from(json!({
            "model": "test",
            "input": "hi",
            "text": { "format": { "type": "json_schema", "name": "out", "schema": { "type": "object" } } }
        }));

        assert!(matches!(
            &prepared.paddler_params.grammar,
            Some(GrammarConstraint::JsonSchema { schema }) if schema == r#"{"type":"object"}"#
        ));
    }

    #[test]
    fn a_request_without_an_output_limit_generates_until_its_context_is_full() {
        let prepared = prepared_from(json!({ "model": "test", "input": "hi" }));

        assert_eq!(prepared.paddler_params.max_tokens, NonZeroU32::MAX);
    }

    #[test]
    fn reasoning_effort_none_disables_thinking() {
        let prepared = prepared_from(json!({
            "model": "test",
            "input": "hi",
            "reasoning": { "effort": "none" }
        }));

        assert!(!prepared.paddler_params.enable_thinking);
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
            let rejection = from_value::<OpenAIResponsesRequestParams>(unrepresentable_request)
                .err()
                .map(|deserialization_error| deserialization_error.classify());

            assert_eq!(rejection, Some(Category::Data));
        }
    }

    #[test]
    fn unsupported_and_stateful_fields_are_ignored() {
        let prepared = prepared_from(json!({
            "model": "test",
            "input": "hi",
            "store": true,
            "previous_response_id": "resp_prev",
            "conversation": "conv_1",
            "temperature": 0.5,
            "tool_choice": "required"
        }));

        assert_eq!(
            prepared.paddler_params.conversation_history.messages.len(),
            1
        );
    }

    #[test]
    fn instructions_alone_become_the_whole_conversation() {
        let prepared = prepared_from(json!({ "model": "test", "instructions": "be terse" }));

        let messages = &prepared.paddler_params.conversation_history.messages;

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[0].content.text_content(), "be terse");
    }

    #[test]
    fn message_item_without_a_type_becomes_a_conversation_message() {
        let prepared = prepared_from(json!({
            "model": "test",
            "input": [ { "role": "user", "content": "hi" } ]
        }));

        let messages = &prepared.paddler_params.conversation_history.messages;

        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[0].content.text_content(), "hi");
    }

    #[test]
    fn rejects_a_function_tool_that_requires_an_undeclared_property() {
        let params: OpenAIResponsesRequestParams = from_value(json!({
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

        let rejection = params.into_prepared().err().unwrap();

        assert_eq!(
            rejection.downcast_ref::<RequestParamsValidationError>(),
            Some(
                &RequestParamsValidationError::RequiredFieldNotInProperties {
                    field: "absent".to_owned(),
                }
            )
        );
    }
}
