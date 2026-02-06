use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;

use super::parameters::Parameters;
use crate::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;
use crate::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use crate::validates::Validates;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Function<TParametersSchema: Default> {
    pub name: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Parameters::is_empty")]
    pub parameters: Parameters<TParametersSchema>,
}

impl Validates<Function<ValidatedParametersSchema>> for Function<RawParametersSchema> {
    fn validate(self) -> Result<Function<ValidatedParametersSchema>> {
        Ok(Function {
            name: self.name,
            description: self.description,
            parameters: self.parameters.validate()?,
        })
    }
}
