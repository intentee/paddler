mod function;
mod parameters;
pub mod parameters_schema;

use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;

use crate::validates::Validates;
use self::function::Function;
use crate::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;
use crate::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionCall<TParametersSchema: Default> {
    pub function: Function<TParametersSchema>,
}

impl Validates<FunctionCall<ValidatedParametersSchema>> for FunctionCall<RawParametersSchema> {
    fn validate(self) -> Result<FunctionCall<ValidatedParametersSchema>> {
        Ok(FunctionCall {
            function: self.function.validate()?,
        })
    }
}
