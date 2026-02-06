pub mod tool;

use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;

use self::tool::Tool;
use crate::validates::Validates;
use crate::conversation_message::ConversationMessage;
use crate::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;
use crate::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContinueFromConversationHistoryParams<TParametersSchema: Default> {
    pub add_generation_prompt: bool,
    pub conversation_history: Vec<ConversationMessage>,
    pub enable_thinking: bool,
    pub max_tokens: i32,
    #[serde(default)]
    pub tools: Vec<Tool<TParametersSchema>>,
}

impl Validates<ContinueFromConversationHistoryParams<ValidatedParametersSchema>>
    for ContinueFromConversationHistoryParams<RawParametersSchema>
{
    fn validate(self) -> Result<ContinueFromConversationHistoryParams<ValidatedParametersSchema>> {
        Ok(ContinueFromConversationHistoryParams {
            add_generation_prompt: self.add_generation_prompt,
            conversation_history: self.conversation_history,
            enable_thinking: self.enable_thinking,
            max_tokens: self.max_tokens,
            tools: self
                .tools
                .into_iter()
                .map(|tool| tool.validate())
                .collect::<Result<Vec<_>>>()?,
        })
    }
}
