use anyhow::Result;
use anyhow::anyhow;
use jsonschema::Validator;
use serde_json::Value;

use crate::official_openai_spec::OPENAPI_YAML;
use crate::official_openai_spec::parse_components;
use crate::openai_conformance_error::OpenAIConformanceError;
use crate::strict_chat_completion_schema::strict_chat_completion_schema;

const REQUEST_ROOT: &str = "CreateChatCompletionRequest";
const RESPONSE_ROOT: &str = "CreateChatCompletionResponse";
const STREAM_ROOT: &str = "CreateChatCompletionStreamResponse";

const REQUEST_STRICT_POINTERS: &[&str] = &["/$defs/CreateChatCompletionRequest"];
const RESPONSE_STRICT_POINTERS: &[&str] = &[
    "/$defs/CreateChatCompletionResponse",
    "/$defs/ChatCompletionResponseMessage",
    "/$defs/CompletionUsage",
    "/$defs/CompletionUsage/properties/prompt_tokens_details",
    "/$defs/CompletionUsage/properties/completion_tokens_details",
];
const STREAM_STRICT_POINTERS: &[&str] = &[
    "/$defs/CreateChatCompletionStreamResponse",
    "/$defs/ChatCompletionStreamResponseDelta",
    "/$defs/CompletionUsage",
    "/$defs/CompletionUsage/properties/prompt_tokens_details",
    "/$defs/CompletionUsage/properties/completion_tokens_details",
];

fn compile_strict_schema(
    components: &Value,
    root_name: &str,
    strict_pointers: &[&str],
) -> Result<Validator> {
    let schema = strict_chat_completion_schema(components, root_name, strict_pointers)?;

    jsonschema::validator_for(&schema)
        .map_err(|error| anyhow!("compiling the strict {root_name:?} schema: {error}"))
}

fn schema_violations(validator: &Validator, instance: &Value) -> Vec<String> {
    validator
        .iter_errors(instance)
        .map(|error| error.to_string())
        .collect()
}

pub struct OpenAIConformanceValidator {
    request: Validator,
    response: Validator,
    stream_chunk: Validator,
}

impl OpenAIConformanceValidator {
    pub fn new() -> Result<Self> {
        Self::from_openapi_yaml(OPENAPI_YAML)
    }

    fn from_openapi_yaml(openapi_yaml: &str) -> Result<Self> {
        Self::from_components(&parse_components(openapi_yaml)?)
    }

    fn from_components(components: &Value) -> Result<Self> {
        Ok(Self {
            request: compile_strict_schema(components, REQUEST_ROOT, REQUEST_STRICT_POINTERS)?,
            response: compile_strict_schema(components, RESPONSE_ROOT, RESPONSE_STRICT_POINTERS)?,
            stream_chunk: compile_strict_schema(components, STREAM_ROOT, STREAM_STRICT_POINTERS)?,
        })
    }

    pub fn validate_chat_completion_request(
        &self,
        instance: &Value,
    ) -> Result<(), OpenAIConformanceError> {
        let violations = schema_violations(&self.request, instance);

        if violations.is_empty() {
            Ok(())
        } else {
            Err(OpenAIConformanceError::RequestDoesNotConform { violations })
        }
    }

    pub fn validate_chat_completion_response(
        &self,
        instance: &Value,
    ) -> Result<(), OpenAIConformanceError> {
        let violations = schema_violations(&self.response, instance);

        if violations.is_empty() {
            Ok(())
        } else {
            Err(OpenAIConformanceError::ResponseDoesNotConform { violations })
        }
    }

    pub fn validate_chat_completion_stream_chunk(
        &self,
        instance: &Value,
    ) -> Result<(), OpenAIConformanceError> {
        let violations = schema_violations(&self.stream_chunk, instance);

        if violations.is_empty() {
            Ok(())
        } else {
            Err(OpenAIConformanceError::StreamChunkDoesNotConform { violations })
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;
    use serde_json::json;

    use super::OpenAIConformanceValidator;
    use super::compile_strict_schema;
    use crate::official_openai_spec::OPENAPI_YAML;
    use crate::official_openai_spec::parse_components;

    fn validator() -> OpenAIConformanceValidator {
        OpenAIConformanceValidator::new().unwrap()
    }

    fn official_request() -> Value {
        json!({
            "model": "test",
            "messages": [{ "role": "user", "content": "Say hello" }]
        })
    }

    fn official_response() -> Value {
        json!({
            "id": "chatcmpl-test",
            "object": "chat.completion",
            "created": 0,
            "model": "test",
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": "hello", "refusal": null },
                "logprobs": null,
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 1,
                "completion_tokens": 1,
                "total_tokens": 2,
                "prompt_tokens_details": { "cached_tokens": 0, "audio_tokens": 0 },
                "completion_tokens_details": { "reasoning_tokens": 0 }
            }
        })
    }

    fn official_stream_chunk() -> Value {
        json!({
            "id": "chatcmpl-test",
            "object": "chat.completion.chunk",
            "created": 0,
            "model": "test",
            "choices": [{
                "index": 0,
                "delta": { "role": "assistant", "content": "hello" },
                "finish_reason": null
            }]
        })
    }

    #[test]
    fn accepts_an_official_request() {
        validator()
            .validate_chat_completion_request(&official_request())
            .unwrap();
    }

    #[test]
    fn rejects_request_with_chat_template_kwargs() {
        let mut request = official_request();
        request["chat_template_kwargs"] = json!({ "enable_thinking": false });

        let error = validator()
            .validate_chat_completion_request(&request)
            .err()
            .unwrap();

        assert!(error.to_string().contains("request does not conform"));
    }

    #[test]
    fn accepts_an_official_response() {
        validator()
            .validate_chat_completion_response(&official_response())
            .unwrap();
    }

    #[test]
    fn rejects_response_with_reasoning_content() {
        let mut response = official_response();
        response["choices"][0]["message"]["reasoning_content"] = json!("thinking");

        let error = validator()
            .validate_chat_completion_response(&response)
            .err()
            .unwrap();

        assert!(error.to_string().contains("response does not conform"));
    }

    #[test]
    fn rejects_response_with_image_tokens() {
        let mut response = official_response();
        response["usage"]["prompt_tokens_details"]["image_tokens"] = json!(3);

        let error = validator()
            .validate_chat_completion_response(&response)
            .err()
            .unwrap();

        assert!(error.to_string().contains("response does not conform"));
    }

    #[test]
    fn accepts_an_official_stream_chunk() {
        validator()
            .validate_chat_completion_stream_chunk(&official_stream_chunk())
            .unwrap();
    }

    #[test]
    fn rejects_stream_chunk_with_reasoning_content() {
        let mut chunk = official_stream_chunk();
        chunk["choices"][0]["delta"]["reasoning_content"] = json!("thinking");

        let error = validator()
            .validate_chat_completion_stream_chunk(&chunk)
            .err()
            .unwrap();

        assert!(error.to_string().contains("stream chunk does not conform"));
    }

    #[test]
    fn rejects_invalid_openapi_yaml() {
        let error = OpenAIConformanceValidator::from_openapi_yaml("key: \"unterminated")
            .err()
            .unwrap();

        assert!(error.to_string().contains("not valid YAML"));
    }

    #[test]
    fn fails_when_request_schema_is_absent() {
        let mut components = parse_components(OPENAPI_YAML).unwrap();
        components
            .as_object_mut()
            .unwrap()
            .remove("CreateChatCompletionRequest");

        let error = OpenAIConformanceValidator::from_components(&components)
            .err()
            .unwrap();

        assert!(error.to_string().contains("CreateChatCompletionRequest"));
    }

    #[test]
    fn fails_when_response_schema_is_absent() {
        let mut components = parse_components(OPENAPI_YAML).unwrap();
        components
            .as_object_mut()
            .unwrap()
            .remove("CreateChatCompletionResponse");

        let error = OpenAIConformanceValidator::from_components(&components)
            .err()
            .unwrap();

        assert!(error.to_string().contains("CreateChatCompletionResponse"));
    }

    #[test]
    fn fails_when_stream_schema_is_absent() {
        let mut components = parse_components(OPENAPI_YAML).unwrap();
        components
            .as_object_mut()
            .unwrap()
            .remove("CreateChatCompletionStreamResponse");

        let error = OpenAIConformanceValidator::from_components(&components)
            .err()
            .unwrap();

        assert!(
            error
                .to_string()
                .contains("CreateChatCompletionStreamResponse")
        );
    }

    #[test]
    fn fails_to_compile_a_structurally_broken_schema() {
        let components = json!({ "Broken": { "$ref": "#/$defs/Missing" } });

        let error = compile_strict_schema(&components, "Broken", &["/$defs/Broken"])
            .err()
            .unwrap();

        assert!(error.to_string().contains("Broken"));
    }
}
