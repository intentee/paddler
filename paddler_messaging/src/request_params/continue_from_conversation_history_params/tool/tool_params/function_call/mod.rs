pub mod function;
pub mod parameters;
pub mod parameters_schema;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;
use crate::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use crate::request_params_validation_error::RequestParamsValidationError;
use crate::validates::Validates;
use self::function::Function;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionCall<TParametersSchema> {
    pub function: Function<TParametersSchema>,
}

impl Validates<FunctionCall<ValidatedParametersSchema>> for FunctionCall<RawParametersSchema> {
    fn validate(
        self,
    ) -> Result<FunctionCall<ValidatedParametersSchema>, RequestParamsValidationError> {
        Ok(FunctionCall {
            function: self.function.validate()?,
        })
    }
}

impl From<&FunctionCall<ValidatedParametersSchema>> for FunctionCall<Value> {
    fn from(FunctionCall { function }: &FunctionCall<ValidatedParametersSchema>) -> Self {
        Self {
            function: Function::from(function),
        }
    }
}
