use anyhow::Context as _;
use anyhow::Result;
use anyhow::bail;
use serde_json::Value;
use yaml_rust2::YamlLoader;

use crate::yaml_to_json_value::yaml_to_json_value;

pub const OPENAPI_YAML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../vendor/openai/openai-openapi/openapi.yaml"
));

pub fn parse_components(openapi_yaml: &str) -> Result<Value> {
    let documents = YamlLoader::load_from_str(openapi_yaml)
        .context("the OpenAI OpenAPI document is not valid YAML")?;

    let document = documents
        .into_iter()
        .next()
        .context("the OpenAI OpenAPI document is empty")?;

    let specification = yaml_to_json_value(&document)?;

    match specification.pointer("/components/schemas") {
        Some(components) => Ok(components.clone()),
        None => bail!("the OpenAI OpenAPI document has no components.schemas object"),
    }
}

#[cfg(test)]
mod tests {
    use super::OPENAPI_YAML;
    use super::parse_components;

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
        assert!(OPENAPI_YAML.contains("reasoning_tokens"));
        assert!(OPENAPI_YAML.contains("service_tier"));
    }

    #[test]
    fn rejects_invalid_yaml() {
        let error = parse_components("key: \"unterminated").unwrap_err();

        assert!(error.to_string().contains("not valid YAML"));
    }

    #[test]
    fn rejects_empty_document() {
        let error = parse_components("").unwrap_err();

        assert!(error.to_string().contains("empty"));
    }

    #[test]
    fn rejects_document_without_components() {
        let error = parse_components("openapi: 3.1.0").unwrap_err();

        assert!(error.to_string().contains("no components.schemas"));
    }

    #[test]
    fn propagates_yaml_conversion_failures() {
        let error = parse_components("1: value").unwrap_err();

        assert!(error.to_string().contains("mapping keys must be strings"));
    }
}
