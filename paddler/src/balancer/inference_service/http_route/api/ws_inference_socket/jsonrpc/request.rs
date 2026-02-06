use serde::Deserialize;
use serde::Serialize;

use crate::request_params::ContinueFromConversationHistoryParams;
use crate::request_params::ContinueFromRawPromptParams;
use crate::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum Request {
    ContinueFromConversationHistory(ContinueFromConversationHistoryParams<RawParametersSchema>),
    ContinueFromRawPrompt(ContinueFromRawPromptParams),
}
