use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;

pub struct ResponsesPreparedRequest {
    pub paddler_params: ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
    pub stream: bool,
    pub model: String,
    pub instructions: Option<String>,
}
