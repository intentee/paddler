pub mod tool;

use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;

use self::tool::Tool;
use crate::conversation_history::ConversationHistory;
use crate::grammar_constraint::GrammarConstraint;
use crate::validates::Validates;
use crate::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;
use crate::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(bound(deserialize = "TParametersSchema: serde::Deserialize<'de>"))]
pub struct ContinueFromConversationHistoryParams<TParametersSchema> {
    pub add_generation_prompt: bool,
    pub conversation_history: ConversationHistory,
    pub enable_thinking: bool,
    #[serde(default)]
    pub grammar: Option<GrammarConstraint>,
    pub max_tokens: i32,
    #[serde(default)]
    pub parse_tool_calls: bool,
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
            grammar: self.grammar,
            max_tokens: self.max_tokens,
            parse_tool_calls: self.parse_tool_calls,
            tools: self
                .tools
                .into_iter()
                .map(Validates::validate)
                .collect::<Result<Vec<_>>>()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::from_value;
    use serde_json::json;

    use super::ContinueFromConversationHistoryParams;
    use crate::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;

    #[test]
    fn a_request_that_omits_the_grammar_field_keeps_working() {
        let request_without_grammar = json!({
            "add_generation_prompt": true,
            "conversation_history": [
                {"content": "Hello", "role": "user"}
            ],
            "enable_thinking": false,
            "max_tokens": 10,
            "tools": [],
        });

        let params: ContinueFromConversationHistoryParams<RawParametersSchema> =
            from_value(request_without_grammar)
                .expect("a request that omits the grammar field must deserialize");

        assert_eq!(params.grammar, None);
    }
}
