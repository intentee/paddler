use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;
use crate::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use crate::request_params_validation_error::RequestParamsValidationError;
use crate::validates::Validates;
use super::parameters::Parameters;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(bound(deserialize = "TParametersSchema: serde::Deserialize<'de>"))]
pub struct Function<TParametersSchema> {
    pub name: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Parameters::is_empty")]
    pub parameters: Parameters<TParametersSchema>,
}

impl Validates<Function<ValidatedParametersSchema>> for Function<RawParametersSchema> {
    fn validate(self) -> Result<Function<ValidatedParametersSchema>, RequestParamsValidationError> {
        Ok(Function {
            name: self.name,
            description: self.description,
            parameters: self.parameters.validate()?,
        })
    }
}

impl From<&Function<ValidatedParametersSchema>> for Function<Value> {
    fn from(
        Function {
            name,
            description,
            parameters,
        }: &Function<ValidatedParametersSchema>,
    ) -> Self {
        Self {
            name: name.clone(),
            description: description.clone(),
            parameters: Parameters::from(parameters),
        }
    }
}
