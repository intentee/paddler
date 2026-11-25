use std::num::NonZeroU32;

use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;

#[must_use]
pub fn greeting_conversation(
    max_tokens: NonZeroU32,
) -> ContinueFromConversationHistoryParams<ValidatedParametersSchema> {
    ContinueFromConversationHistoryParams {
        add_generation_prompt: true,
        conversation_history: ConversationHistory::new(vec![ConversationMessage::Message {
            content: ConversationMessageContent::Text("hi".to_owned()),
            role: "user".to_owned(),
        }]),
        enable_thinking: false,
        grammar: None,
        max_tokens,
        parse_tool_calls: false,
        tools: vec![],
    }
}
