use anyhow::Result;
use anyhow::anyhow;
use jsonschema::Validator;
use jsonschema::validator_for;
use serde_json::Value;

use crate::openai_spec::OPENAPI_YAML;
use crate::openai_spec::parse_components;
use crate::openai_validator_error::OpenAIValidatorError;
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

const RESPONSES_REQUEST_ROOT: &str = "CreateResponse";
const RESPONSES_RESPONSE_ROOT: &str = "Response";
const RESPONSES_STREAM_EVENT_ROOT: &str = "ResponseStreamEvent";

const RESPONSES_REQUEST_STRICT_POINTERS: &[&str] = &[];

const RESPONSES_SHARED_OUTPUT_STRICT_POINTERS: &[&str] = &[
    "/$defs/Response",
    "/$defs/OutputMessage",
    "/$defs/OutputTextContent",
    "/$defs/ReasoningItem",
    "/$defs/FunctionToolCall",
    "/$defs/ResponseUsage",
    "/$defs/ResponseUsage/properties/input_tokens_details",
    "/$defs/ResponseUsage/properties/output_tokens_details",
];

const RESPONSES_EMITTED_EVENT_STRICT_POINTERS: &[&str] = &[
    "/$defs/ResponseCreatedEvent",
    "/$defs/ResponseInProgressEvent",
    "/$defs/ResponseCompletedEvent",
    "/$defs/ResponseFailedEvent",
    "/$defs/ResponseOutputItemAddedEvent",
    "/$defs/ResponseOutputItemDoneEvent",
    "/$defs/ResponseContentPartAddedEvent",
    "/$defs/ResponseContentPartDoneEvent",
    "/$defs/ResponseTextDeltaEvent",
    "/$defs/ResponseTextDoneEvent",
    "/$defs/ResponseReasoningTextDeltaEvent",
    "/$defs/ResponseReasoningTextDoneEvent",
    "/$defs/ResponseFunctionCallArgumentsDeltaEvent",
    "/$defs/ResponseFunctionCallArgumentsDoneEvent",
];

const ERROR_RESPONSE_ROOT: &str = "ErrorResponse";
const ERROR_RESPONSE_STRICT_POINTERS: &[&str] = &["/$defs/ErrorResponse", "/$defs/Error"];

fn responses_stream_event_strict_pointers() -> Vec<&'static str> {
    let mut pointers = RESPONSES_EMITTED_EVENT_STRICT_POINTERS.to_vec();

    pointers.extend_from_slice(RESPONSES_SHARED_OUTPUT_STRICT_POINTERS);

    pointers
}

fn compile_strict_schema(
    components: &Value,
    root_name: &str,
    strict_pointers: &[&str],
) -> Result<Validator> {
    let schema = strict_chat_completion_schema(components, root_name, strict_pointers)?;

    validator_for(&schema)
        .map_err(|error| anyhow!("compiling the strict {root_name:?} schema: {error}"))
}

fn schema_violations(validator: &Validator, instance: &Value) -> Vec<String> {
    validator
        .iter_errors(instance)
        .map(|error| error.to_string())
        .collect()
}

pub struct OpenAIValidator {
    request: Validator,
    response: Validator,
    stream_chunk: Validator,
    responses_request: Validator,
    responses_response: Validator,
    responses_stream_event: Validator,
    error_response: Validator,
}

impl OpenAIValidator {
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
            responses_request: compile_strict_schema(
                components,
                RESPONSES_REQUEST_ROOT,
                RESPONSES_REQUEST_STRICT_POINTERS,
            )?,
            responses_response: compile_strict_schema(
                components,
                RESPONSES_RESPONSE_ROOT,
                RESPONSES_SHARED_OUTPUT_STRICT_POINTERS,
            )?,
            responses_stream_event: compile_strict_schema(
                components,
                RESPONSES_STREAM_EVENT_ROOT,
                &responses_stream_event_strict_pointers(),
            )?,
            error_response: compile_strict_schema(
                components,
                ERROR_RESPONSE_ROOT,
                ERROR_RESPONSE_STRICT_POINTERS,
            )?,
        })
    }

    pub fn validate_chat_completion_request(
        &self,
        instance: &Value,
    ) -> Result<(), OpenAIValidatorError> {
        let violations = schema_violations(&self.request, instance);

        if violations.is_empty() {
            Ok(())
        } else {
            Err(OpenAIValidatorError::RequestDoesNotConform { violations })
        }
    }

    pub fn validate_chat_completion_response(
        &self,
        instance: &Value,
    ) -> Result<(), OpenAIValidatorError> {
        let violations = schema_violations(&self.response, instance);

        if violations.is_empty() {
            Ok(())
        } else {
            Err(OpenAIValidatorError::ResponseDoesNotConform { violations })
        }
    }

    pub fn validate_chat_completion_stream_chunk(
        &self,
        instance: &Value,
    ) -> Result<(), OpenAIValidatorError> {
        let violations = schema_violations(&self.stream_chunk, instance);

        if violations.is_empty() {
            Ok(())
        } else {
            Err(OpenAIValidatorError::StreamChunkDoesNotConform { violations })
        }
    }

    pub fn validate_responses_request(&self, instance: &Value) -> Result<(), OpenAIValidatorError> {
        let violations = schema_violations(&self.responses_request, instance);

        if violations.is_empty() {
            Ok(())
        } else {
            Err(OpenAIValidatorError::ResponsesRequestDoesNotConform { violations })
        }
    }

    pub fn validate_responses_response(
        &self,
        instance: &Value,
    ) -> Result<(), OpenAIValidatorError> {
        let violations = schema_violations(&self.responses_response, instance);

        if violations.is_empty() {
            Ok(())
        } else {
            Err(OpenAIValidatorError::ResponsesResponseDoesNotConform { violations })
        }
    }

    pub fn validate_responses_stream_event(
        &self,
        instance: &Value,
    ) -> Result<(), OpenAIValidatorError> {
        let violations = schema_violations(&self.responses_stream_event, instance);

        if violations.is_empty() {
            Ok(())
        } else {
            Err(OpenAIValidatorError::ResponsesStreamEventDoesNotConform { violations })
        }
    }

    pub fn validate_error_response(&self, instance: &Value) -> Result<(), OpenAIValidatorError> {
        let violations = schema_violations(&self.error_response, instance);

        if violations.is_empty() {
            Ok(())
        } else {
            Err(OpenAIValidatorError::ErrorResponseDoesNotConform { violations })
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Context as _;
    use anyhow::Result;
    use serde_json::Value;
    use serde_json::json;

    use super::OpenAIValidator;
    use super::compile_strict_schema;
    use crate::openai_spec::OPENAPI_YAML;
    use crate::openai_spec::parse_components;

    fn validator() -> OpenAIValidator {
        OpenAIValidator::new().unwrap()
    }

    fn conformant_request() -> Value {
        json!({
            "model": "test",
            "messages": [{ "role": "user", "content": "Say hello" }]
        })
    }

    fn conformant_response() -> Value {
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

    fn conformant_stream_chunk() -> Value {
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
    fn accepts_a_conformant_request() {
        validator()
            .validate_chat_completion_request(&conformant_request())
            .unwrap();
    }

    #[test]
    fn rejects_request_with_chat_template_kwargs() {
        let mut request = conformant_request();
        request["chat_template_kwargs"] = json!({ "enable_thinking": false });

        let error = validator()
            .validate_chat_completion_request(&request)
            .err()
            .unwrap();

        assert!(error.to_string().contains("request does not conform"));
    }

    #[test]
    fn accepts_a_conformant_response() {
        validator()
            .validate_chat_completion_response(&conformant_response())
            .unwrap();
    }

    #[test]
    fn rejects_response_with_reasoning_content() {
        let mut response = conformant_response();
        response["choices"][0]["message"]["reasoning_content"] = json!("thinking");

        let error = validator()
            .validate_chat_completion_response(&response)
            .err()
            .unwrap();

        assert!(error.to_string().contains("response does not conform"));
    }

    #[test]
    fn rejects_response_with_image_tokens() {
        let mut response = conformant_response();
        response["usage"]["prompt_tokens_details"]["image_tokens"] = json!(3);

        let error = validator()
            .validate_chat_completion_response(&response)
            .err()
            .unwrap();

        assert!(error.to_string().contains("response does not conform"));
    }

    #[test]
    fn accepts_a_conformant_stream_chunk() {
        validator()
            .validate_chat_completion_stream_chunk(&conformant_stream_chunk())
            .unwrap();
    }

    #[test]
    fn rejects_stream_chunk_with_reasoning_content() {
        let mut chunk = conformant_stream_chunk();
        chunk["choices"][0]["delta"]["reasoning_content"] = json!("thinking");

        let error = validator()
            .validate_chat_completion_stream_chunk(&chunk)
            .err()
            .unwrap();

        assert!(error.to_string().contains("stream chunk does not conform"));
    }

    #[test]
    fn rejects_invalid_openapi_yaml() {
        let error = OpenAIValidator::from_openapi_yaml("key: \"unterminated")
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

        let error = OpenAIValidator::from_components(&components).err().unwrap();

        assert!(error.to_string().contains("CreateChatCompletionRequest"));
    }

    #[test]
    fn fails_when_response_schema_is_absent() {
        let mut components = parse_components(OPENAPI_YAML).unwrap();
        components
            .as_object_mut()
            .unwrap()
            .remove("CreateChatCompletionResponse");

        let error = OpenAIValidator::from_components(&components).err().unwrap();

        assert!(error.to_string().contains("CreateChatCompletionResponse"));
    }

    #[test]
    fn fails_when_stream_schema_is_absent() {
        let mut components = parse_components(OPENAPI_YAML).unwrap();
        components
            .as_object_mut()
            .unwrap()
            .remove("CreateChatCompletionStreamResponse");

        let error = OpenAIValidator::from_components(&components).err().unwrap();

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

    fn conformant_responses_request() -> Value {
        json!({ "model": "test", "input": "Say hello" })
    }

    fn conformant_responses_response() -> Value {
        json!({
            "id": "resp_test",
            "object": "response",
            "created_at": 0,
            "error": null,
            "incomplete_details": null,
            "instructions": null,
            "model": "test",
            "tools": [],
            "output": [{
                "id": "msg_0",
                "type": "message",
                "role": "assistant",
                "status": "completed",
                "content": [{
                    "type": "output_text",
                    "text": "hello",
                    "annotations": [],
                    "logprobs": []
                }]
            }],
            "parallel_tool_calls": true,
            "metadata": {},
            "tool_choice": "auto",
            "temperature": 1,
            "top_p": 1,
            "usage": {
                "input_tokens": 1,
                "input_tokens_details": { "cached_tokens": 0 },
                "output_tokens": 1,
                "output_tokens_details": { "reasoning_tokens": 0 },
                "total_tokens": 2
            }
        })
    }

    fn conformant_responses_stream_event() -> Value {
        json!({
            "type": "response.output_text.delta",
            "item_id": "msg_0",
            "output_index": 0,
            "content_index": 0,
            "delta": "hello",
            "sequence_number": 1,
            "logprobs": []
        })
    }

    #[test]
    fn accepts_a_conformant_responses_request() {
        validator()
            .validate_responses_request(&conformant_responses_request())
            .unwrap();
    }

    #[test]
    fn accepts_a_conformant_responses_response() {
        validator()
            .validate_responses_response(&conformant_responses_response())
            .unwrap();
    }

    #[test]
    fn rejects_responses_response_with_an_extra_top_level_key() {
        let mut response = conformant_responses_response();
        response["paddler_extension"] = json!("nope");

        let error = validator()
            .validate_responses_response(&response)
            .err()
            .unwrap();

        assert!(
            error
                .to_string()
                .contains("responses response does not conform")
        );
    }

    #[test]
    fn rejects_responses_response_with_an_extra_output_text_field() {
        let mut response = conformant_responses_response();
        response["output"][0]["content"][0]["reasoning_content"] = json!("nope");

        let error = validator()
            .validate_responses_response(&response)
            .err()
            .unwrap();

        assert!(
            error
                .to_string()
                .contains("responses response does not conform")
        );
    }

    #[test]
    fn accepts_a_conformant_responses_stream_event() {
        validator()
            .validate_responses_stream_event(&conformant_responses_stream_event())
            .unwrap();
    }

    #[test]
    fn rejects_responses_stream_event_with_an_extra_key() {
        let mut event = conformant_responses_stream_event();
        event["paddler_extension"] = json!("nope");

        let error = validator()
            .validate_responses_stream_event(&event)
            .err()
            .unwrap();

        assert!(
            error
                .to_string()
                .contains("responses stream event does not conform")
        );
    }

    #[test]
    fn fails_when_create_response_schema_is_absent() {
        let mut components = parse_components(OPENAPI_YAML).unwrap();
        components.as_object_mut().unwrap().remove("CreateResponse");

        let error = OpenAIValidator::from_components(&components).err().unwrap();

        assert!(error.to_string().contains("CreateResponse"));
    }

    #[test]
    fn fails_when_responses_response_schema_is_absent() {
        let mut components = parse_components(OPENAPI_YAML).unwrap();
        components.as_object_mut().unwrap().remove("Response");

        let error = OpenAIValidator::from_components(&components).err().unwrap();

        assert!(error.to_string().contains("Response"));
    }

    #[test]
    fn fails_when_response_stream_event_schema_is_absent() {
        let mut components = parse_components(OPENAPI_YAML).unwrap();
        components
            .as_object_mut()
            .unwrap()
            .remove("ResponseStreamEvent");

        let error = OpenAIValidator::from_components(&components).err().unwrap();

        assert!(error.to_string().contains("ResponseStreamEvent"));
    }

    fn conformant_error_response() -> Value {
        json!({
            "error": {
                "message": "boom",
                "type": "server_error",
                "param": null,
                "code": null
            }
        })
    }

    #[test]
    fn accepts_a_conformant_error_response() -> Result<()> {
        OpenAIValidator::new()?.validate_error_response(&conformant_error_response())?;

        Ok(())
    }

    #[test]
    fn rejects_error_response_with_an_extra_key() -> Result<()> {
        let mut response = conformant_error_response();
        response["error"]["paddler_extension"] = json!("nope");

        match OpenAIValidator::new()?.validate_error_response(&response) {
            Ok(()) => panic!("expected the error response to be rejected"),
            Err(error) => assert!(
                error
                    .to_string()
                    .contains("error response does not conform")
            ),
        }

        Ok(())
    }

    #[test]
    fn rejects_error_response_missing_a_required_field() -> Result<()> {
        let response = json!({
            "error": {
                "message": "boom",
                "param": null,
                "code": null
            }
        });

        match OpenAIValidator::new()?.validate_error_response(&response) {
            Ok(()) => panic!("expected the error response to be rejected"),
            Err(error) => assert!(
                error
                    .to_string()
                    .contains("error response does not conform")
            ),
        }

        Ok(())
    }

    #[test]
    fn fails_when_error_response_schema_is_absent() -> Result<()> {
        let mut components = parse_components(OPENAPI_YAML)?;
        components
            .as_object_mut()
            .context("parsed components must be a JSON object")?
            .remove("ErrorResponse");

        match OpenAIValidator::from_components(&components) {
            Ok(_) => panic!("expected schema compilation to fail without ErrorResponse"),
            Err(error) => assert!(error.to_string().contains("ErrorResponse")),
        }

        Ok(())
    }
}
