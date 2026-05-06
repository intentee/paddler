use serde::Deserialize;
use serde::Serialize;

use crate::request_params::ContinueFromRawPromptParams;
use crate::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum Request<TParametersSchema> {
    ContinueFromConversationHistory(ContinueFromConversationHistoryParams<TParametersSchema>),
    ContinueFromRawPrompt(ContinueFromRawPromptParams),
}
