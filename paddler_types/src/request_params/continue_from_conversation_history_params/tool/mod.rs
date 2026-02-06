pub mod tool_params;

use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;

use self::tool_params::FunctionCall;
use crate::validates::Validates;
use crate::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;
use crate::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(tag = "type")]
pub enum Tool<TParametersSchema: Default> {
    #[serde(rename = "function")]
    Function(FunctionCall<TParametersSchema>),
}

impl Validates<Tool<ValidatedParametersSchema>> for Tool<RawParametersSchema> {
    fn validate(self) -> Result<Tool<ValidatedParametersSchema>> {
        match self {
            Tool::Function(function_call) => Ok(Tool::Function(function_call.validate()?)),
        }
    }
}
