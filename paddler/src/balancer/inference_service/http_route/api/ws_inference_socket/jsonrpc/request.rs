use serde::Deserialize;
use serde::Serialize;

use paddler_types::request_params::ContinueFromConversationHistoryParams;
use paddler_types::request_params::ContinueFromRawPromptParams;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum Request {
    ContinueFromConversationHistory(ContinueFromConversationHistoryParams<RawParametersSchema>),
    ContinueFromRawPrompt(ContinueFromRawPromptParams),
}
