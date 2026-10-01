use serde_json::Value;
use yaml_rust2::YamlLoader;

use crate::openai_validator_error::OpenAIValidatorError;
use crate::yaml_to_json_value::yaml_to_json_value;

pub const OPENAPI_YAML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../vendor/openai/openai-openapi/openapi.yaml"
));

pub fn parse_components(openapi_yaml: &str) -> Result<Value, OpenAIValidatorError> {
    let documents =
        YamlLoader::load_from_str(openapi_yaml).map_err(OpenAIValidatorError::SpecNotValidYaml)?;

    let document = documents
        .into_iter()
        .next()
        .ok_or(OpenAIValidatorError::SpecEmpty)?;

    let specification = yaml_to_json_value(&document)?;

    specification
        .pointer("/components/schemas")
        .cloned()
        .ok_or(OpenAIValidatorError::SpecWithoutComponentSchemas)
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;

    use yaml_rust2::YamlLoader;
    use yaml_rust2::yaml::Yaml;

    use super::OPENAPI_YAML;
    use super::parse_components;
    use crate::openai_validator_error::OpenAIValidatorError;

    #[test]
    fn parses_the_embedded_spec_components() {
        let components = parse_components(OPENAPI_YAML).unwrap();

        assert!(components.get("CreateChatCompletionRequest").is_some());
        assert!(components.get("CreateChatCompletionResponse").is_some());
        assert!(
            components
                .get("CreateChatCompletionStreamResponse")
                .is_some()
        );
    }

    #[test]
    fn the_embedded_spec_is_the_modern_3_1_spec() {
        let documents = YamlLoader::load_from_str(OPENAPI_YAML).unwrap();
        let components = parse_components(OPENAPI_YAML).unwrap();

        assert_eq!(documents[0]["openapi"].as_str(), Some("3.1.0"));
        assert!(
            components
                .pointer("/CompletionUsage/properties/completion_tokens_details/properties/reasoning_tokens")
                .is_some()
        );
        assert!(
            components
                .pointer("/CreateChatCompletionResponse/properties/service_tier")
                .is_some()
        );
    }

    #[test]
    fn rejects_invalid_yaml() {
        let error = parse_components("key: \"unterminated").unwrap_err();

        assert!(matches!(
            error,
            OpenAIValidatorError::SpecNotValidYaml(ref scan_error)
                if *scan_error == YamlLoader::load_from_str("key: \"unterminated").unwrap_err()
        ));
    }

    #[test]
    fn rejects_empty_document() {
        let error = parse_components("").unwrap_err();

        assert_eq!(
            discriminant(&error),
            discriminant(&OpenAIValidatorError::SpecEmpty)
        );
    }

    #[test]
    fn rejects_document_without_components() {
        let error = parse_components("openapi: 3.1.0").unwrap_err();

        assert_eq!(
            discriminant(&error),
            discriminant(&OpenAIValidatorError::SpecWithoutComponentSchemas)
        );
    }

    #[test]
    fn propagates_yaml_conversion_failures() {
        let error = parse_components("1: value").unwrap_err();

        assert!(matches!(
            error,
            OpenAIValidatorError::YamlMappingKeyNotString { ref key } if *key == Yaml::Integer(1)
        ));
    }
}
