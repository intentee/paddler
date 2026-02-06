use serde::Deserialize;
use serde::Serialize;

use crate::request_params::ContinueFromConversationHistoryParams;
use crate::request_params::ContinueFromRawPromptParams;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum Request<TParametersSchema: Default> {
    ContinueFromConversationHistory(ContinueFromConversationHistoryParams<TParametersSchema>),
    ContinueFromRawPrompt(ContinueFromRawPromptParams),
}
