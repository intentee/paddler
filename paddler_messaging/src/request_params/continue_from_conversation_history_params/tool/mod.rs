pub mod tool_params;

use serde::Deserialize;
use serde::Serialize;

use crate::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;
use crate::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use crate::request_params_validation_error::RequestParamsValidationError;
use crate::validates::Validates;
use self::tool_params::function_call::FunctionCall;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(tag = "type")]
pub enum Tool<TParametersSchema> {
    #[serde(rename = "function")]
    Function(FunctionCall<TParametersSchema>),
}

impl Validates<Tool<ValidatedParametersSchema>> for Tool<RawParametersSchema> {
    fn validate(self) -> Result<Tool<ValidatedParametersSchema>, RequestParamsValidationError> {
        match self {
            Self::Function(function_call) => Ok(Tool::Function(function_call.validate()?)),
        }
    }
}
