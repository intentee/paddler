use std::num::ParseFloatError;

use jsonschema::ValidationError;
use serde_json::Value;
use thiserror::Error;
use yaml_rust2::ScanError;
use yaml_rust2::yaml::Yaml;

#[derive(Debug, Error)]
pub enum OpenAIValidatorError {
    #[error("chat completion request does not conform to the OpenAI schema: {violations:?}")]
    RequestDoesNotConform { violations: Vec<String> },
    #[error("chat completion response does not conform to the OpenAI schema: {violations:?}")]
    ResponseDoesNotConform { violations: Vec<String> },
    #[error("chat completion stream chunk does not conform to the OpenAI schema: {violations:?}")]
    StreamChunkDoesNotConform { violations: Vec<String> },
    #[error("responses request does not conform to the OpenAI schema: {violations:?}")]
    ResponsesRequestDoesNotConform { violations: Vec<String> },
    #[error("responses response does not conform to the OpenAI schema: {violations:?}")]
    ResponsesResponseDoesNotConform { violations: Vec<String> },
    #[error("responses stream event does not conform to the OpenAI schema: {violations:?}")]
    ResponsesStreamEventDoesNotConform { violations: Vec<String> },
    #[error("error response does not conform to the OpenAI schema: {violations:?}")]
    ErrorResponseDoesNotConform { violations: Vec<String> },
    #[error("the OpenAI OpenAPI document is not valid YAML")]
    SpecNotValidYaml(#[source] ScanError),
    #[error("the OpenAI OpenAPI document is empty")]
    SpecEmpty,
    #[error("the OpenAI OpenAPI document has no components.schemas object")]
    SpecWithoutComponentSchemas,
    #[error("could not parse YAML real {real:?}")]
    YamlRealUnparsable {
        real: String,
        #[source]
        source: ParseFloatError,
    },
    #[error("YAML real {real:?} is not finite")]
    YamlRealNotFinite { real: String },
    #[error("YAML mapping keys must be strings, found {key:?}")]
    YamlMappingKeyNotString { key: Yaml },
    #[error("YAML aliases are not supported (alias #{index})")]
    YamlAliasUnsupported { index: usize },
    #[error("encountered an invalid YAML node")]
    YamlBadValue,
    #[error("schema references unknown component {name:?}")]
    UnknownComponent { name: String },
    #[error("strict target {pointer:?} is not an object: {target}")]
    StrictTargetNotAnObject { pointer: String, target: Value },
    #[error("strict target {pointer:?} was not found in the assembled schema")]
    StrictTargetNotFound { pointer: String },
    #[error("compiling the strict {root_name:?} schema failed")]
    StrictSchemaDoesNotCompile {
        root_name: String,
        #[source]
        source: Box<ValidationError<'static>>,
    },
}
