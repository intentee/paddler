use anyhow::Result;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::FunctionCall;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::function::Function;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters::Parameters;
use paddler_messaging::validates::Validates;
use serde::Deserialize;

use crate::compatibility::openai_service::openai_responses_function_tool::OpenAIResponsesFunctionTool;
use crate::compatibility::openai_service::openai_responses_input::OpenAIResponsesInput;
use crate::compatibility::openai_service::openai_responses_input_item::OpenAIResponsesInputItem;
use crate::compatibility::openai_service::openai_responses_reasoning::OpenAIResponsesReasoning;
use crate::compatibility::openai_service::openai_responses_text_param::OpenAIResponsesTextParam;
use crate::compatibility::openai_service::openai_responses_tool::OpenAIResponsesTool;
use crate::compatibility::openai_service::responses_prepared_request::ResponsesPreparedRequest;

const DEFAULT_MAX_TOKENS: i32 = 2000;

#[derive(Deserialize)]
pub struct OpenAIResponsesRequestParams {
    /// Echoed back in the response object; not used for routing.
    pub model: String,
    #[serde(default)]
    pub input: OpenAIResponsesInput,
    #[serde(default)]
    pub instructions: Option<String>,
    #[serde(default)]
    pub stream: Option<bool>,
    #[serde(default)]
    pub max_output_tokens: Option<i32>,
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
                        .filter_map(OpenAIResponsesInputItem::into_conversation_message),
                );
            }
        }

        let validated_tools = tools
            .into_iter()
            .filter_map(|tool| match tool {
                OpenAIResponsesTool::Function(function_tool) => {
                    let OpenAIResponsesFunctionTool {
                        name,
                        description,
                        parameters,
                    } = *function_tool;

                    Some(Tool::Function(FunctionCall {
                        function: Function {
                            name,
                            description: description.unwrap_or_default(),
                            parameters: parameters.map_or(Parameters::Empty, Parameters::Schema),
                        },
                    }))
                }
                OpenAIResponsesTool::Unsupported => None,
            })
            .map(Validates::validate)
            .collect::<Result<Vec<_>>>()?;

        let parse_tool_calls = !validated_tools.is_empty();

        Ok(ResponsesPreparedRequest {
            paddler_params: ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(messages),
                enable_thinking: reasoning
                    .as_ref()
                    .is_none_or(OpenAIResponsesReasoning::enables_thinking),
                grammar: match text {
                    Some(text_param) => text_param.into_grammar_constraint()?,
                    None => None,
                },
                max_tokens: max_output_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
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
    use paddler_messaging::grammar_constraint::GrammarConstraint;
    use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
    use serde_json::json;

    use super::OpenAIResponsesRequestParams;
    use crate::compatibility::openai_service::responses_prepared_request::ResponsesPreparedRequest;

    fn prepared_from(value: serde_json::Value) -> ResponsesPreparedRequest {
        let params: OpenAIResponsesRequestParams = serde_json::from_value(value).unwrap();

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

        let Some(GrammarConstraint::JsonSchema { schema }) = &prepared.paddler_params.grammar
        else {
            panic!("expected a json schema grammar constraint");
        };

        assert!(schema.contains("\"type\":\"object\""));
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
    fn unsupported_tool_is_skipped_and_disables_tool_call_parsing() {
        let prepared = prepared_from(json!({
            "model": "test",
            "input": "hi",
            "tools": [ { "type": "web_search" } ]
        }));

        assert!(prepared.paddler_params.tools.is_empty());
        assert!(!prepared.paddler_params.parse_tool_calls);
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
}
