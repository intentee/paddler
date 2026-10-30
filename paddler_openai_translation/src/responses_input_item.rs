use serde::Deserialize;

use paddler_messaging::assistant_role::ASSISTANT_ROLE;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;

use crate::openai_translation_error::OpenAITranslationError;
use crate::responses_function_call_output_item::ResponsesFunctionCallOutputItem;
use crate::responses_message_item::ResponsesMessageItem;
use crate::responses_tagged_item::ResponsesTaggedItem;

#[derive(Deserialize)]
#[serde(untagged)]
pub enum ResponsesInputItem {
    Tagged(ResponsesTaggedItem),
    Message(ResponsesMessageItem),
}

impl ResponsesInputItem {
    pub fn append_to(
        self,
        messages: &mut Vec<ConversationMessage>,
    ) -> Result<(), OpenAITranslationError> {
        match self {
            Self::Message(message) | Self::Tagged(ResponsesTaggedItem::Message(message)) => {
                messages.push(message.into_conversation_message());
            }
            Self::Tagged(ResponsesTaggedItem::FunctionCall(function_call)) => {
                let tool_call = function_call.into_conversation_tool_call()?;

                let assistant_tool_calls_turn = match messages.pop() {
                    Some(ConversationMessage::AssistantToolCalls {
                        content,
                        mut tool_calls,
                    }) => {
                        tool_calls.push(tool_call);

                        ConversationMessage::AssistantToolCalls {
                            content,
                            tool_calls,
                        }
                    }
                    Some(ConversationMessage::Message { content, role })
                        if role == ASSISTANT_ROLE =>
                    {
                        ConversationMessage::AssistantToolCalls {
                            content,
                            tool_calls: vec![tool_call],
                        }
                    }
                    preceding_message => {
                        messages.extend(preceding_message);

                        ConversationMessage::AssistantToolCalls {
                            content: ConversationMessageContent::Text(String::new()),
                            tool_calls: vec![tool_call],
                        }
                    }
                };

                messages.push(assistant_tool_calls_turn);
            }
            Self::Tagged(ResponsesTaggedItem::FunctionCallOutput(
                ResponsesFunctionCallOutputItem { call_id, output },
            )) => messages.push(ConversationMessage::ToolResult {
                content: ConversationMessageContent::Text(output.into_text()),
                tool_call_id: call_id,
            }),
            Self::Tagged(ResponsesTaggedItem::Reasoning(reasoning)) => {
                messages.push(reasoning.into_conversation_message()?);
            }
        }

        Ok(())
    }
}
