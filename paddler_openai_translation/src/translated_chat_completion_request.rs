use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;

use crate::chat_completion_delivery::ChatCompletionDelivery;

pub struct TranslatedChatCompletionRequest {
    pub conversation_history_params:
        ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
    pub delivery: ChatCompletionDelivery,
}
