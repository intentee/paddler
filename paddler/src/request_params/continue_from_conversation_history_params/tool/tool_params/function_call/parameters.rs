use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;

use crate::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;
use crate::validates::Validates;
use crate::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Parameters<TParametersSchema: Default> {
    #[default]
    Empty,
    Schema(TParametersSchema),
}

impl<TParametersSchema: Default> Parameters<TParametersSchema> {
    pub fn is_empty(&self) -> bool {
        matches!(self, Parameters::Empty)
    }
}

impl Validates<Parameters<ValidatedParametersSchema>> for Parameters<RawParametersSchema> {
    fn validate(self) -> Result<Parameters<ValidatedParametersSchema>> {
        match self {
            Parameters::Empty => Ok(Parameters::Empty),
            Parameters::Schema(schema) => Ok(Parameters::Schema(schema.validate()?)),
        }
    }
}
